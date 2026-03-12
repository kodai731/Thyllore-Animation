use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use ort::session::Session;
use ort::value::Tensor;

use super::inference_request::{
    InferenceActorId, InferenceRequest, InferenceRequestKind, InferenceResult, InferenceResultKind,
};

pub struct InferenceThreadHandle {
    sender: Option<mpsc::Sender<InferenceRequest>>,
    receiver: mpsc::Receiver<InferenceResult>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl InferenceThreadHandle {
    pub fn spawn(model_path: &str, actor_id: InferenceActorId) -> Result<Self> {
        let (req_tx, req_rx) = mpsc::channel::<InferenceRequest>();
        let (res_tx, res_rx) = mpsc::channel::<InferenceResult>();

        let session = Session::builder()?
            .with_intra_threads(2)?
            .with_inter_threads(1)?
            .commit_from_file(model_path)?;

        let join_handle = thread::Builder::new()
            .name(format!("inference-actor-{}", actor_id))
            .spawn(move || {
                run_inference_loop(session, req_rx, res_tx);
            })?;

        Ok(Self {
            sender: Some(req_tx),
            receiver: res_rx,
            join_handle: Some(join_handle),
        })
    }

    pub fn send(&self, request: InferenceRequest) -> Result<()> {
        if let Some(ref sender) = self.sender {
            sender.send(request)?;
        }
        Ok(())
    }

    pub fn try_recv(&self) -> Option<InferenceResult> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for InferenceThreadHandle {
    fn drop(&mut self) {
        self.sender.take();

        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

fn run_inference_loop(
    mut session: Session,
    receiver: mpsc::Receiver<InferenceRequest>,
    sender: mpsc::Sender<InferenceResult>,
) {
    while let Ok(request) = receiver.recv() {
        let result = execute_inference(&mut session, &request);

        match result {
            Ok(result_kind) => {
                let response = InferenceResult {
                    request_id: request.request_id,
                    actor_id: request.actor_id,
                    kind: result_kind,
                };
                if sender.send(response).is_err() {
                    break;
                }
            }
            Err(e) => {
                crate::log!("Inference error for actor {}: {:?}", request.actor_id, e);
            }
        }
    }
}

fn execute_inference(
    session: &mut Session,
    request: &InferenceRequest,
) -> Result<InferenceResultKind> {
    match &request.kind {
        InferenceRequestKind::CurvePredict { input } => {
            let input_len = input.len();
            let input_tensor = Tensor::from_array((vec![1i64, input_len as i64], input.clone()))?;

            let outputs = session.run(ort::inputs![input_tensor])?;

            let (_shape, output_data) = outputs[0].try_extract_tensor::<f32>()?;
            let output_vec: Vec<f32> = output_data.to_vec();

            Ok(InferenceResultKind::CurvePredict { output: output_vec })
        }

        InferenceRequestKind::CurveCopilotPredict {
            context,
            property_type_id,
            topology_features,
            bone_name_tokens,
            query_times,
            curve_window,
        } => execute_curve_copilot(
            session,
            context,
            *property_type_id,
            topology_features,
            bone_name_tokens,
            query_times,
            curve_window,
        ),
    }
}

fn execute_curve_copilot(
    session: &mut Session,
    context: &[f32],
    property_type_id: u32,
    topology_features: &[f32],
    bone_name_tokens: &[i64],
    query_times: &[f32],
    curve_window: &[f32],
) -> Result<InferenceResultKind> {
    use super::inference_request::CopilotStepPrediction;

    let num_steps = query_times.len() as i64;

    let context_tensor = Tensor::from_array((vec![1i64, 8, 6], context.to_vec()))?;
    let property_type_tensor = Tensor::from_array((vec![1i64], vec![property_type_id as i64]))?;
    let topology_tensor = Tensor::from_array((vec![1i64, 6], topology_features.to_vec()))?;
    let name_tensor = Tensor::from_array((vec![1i64, 32], bone_name_tokens.to_vec()))?;
    let query_times_tensor = Tensor::from_array((vec![1i64, num_steps], query_times.to_vec()))?;
    let curve_window_len = curve_window.len() as i64;
    let curve_window_tensor =
        Tensor::from_array((vec![1i64, curve_window_len], curve_window.to_vec()))?;

    let outputs = session.run(ort::inputs![
        "context_keyframes" => context_tensor,
        "property_type" => property_type_tensor,
        "topology_features" => topology_tensor,
        "bone_name_tokens" => name_tensor,
        "query_times" => query_times_tensor,
        "curve_window" => curve_window_tensor
    ])?;

    let (_shape, prediction_data) = outputs[0].try_extract_tensor::<f32>()?;
    let pred: Vec<f32> = prediction_data.to_vec();

    let (_shape, confidence_data) = outputs[1].try_extract_tensor::<f32>()?;
    let conf_raw: Vec<f32> = confidence_data.to_vec();

    crate::log!("CurveCopilot raw confidence (pre-sigmoid): {:?}", &conf_raw);

    let conf: Vec<f32> = conf_raw.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect();

    const VALUES_PER_STEP: usize = 5;
    let step_count = num_steps as usize;
    let mut steps = Vec::with_capacity(step_count);

    for i in 0..step_count {
        let offset = i * VALUES_PER_STEP;
        if offset + VALUES_PER_STEP > pred.len() {
            break;
        }
        let value = pred[offset];
        let tangent_in = (pred[offset + 1], pred[offset + 2]);
        let tangent_out = (pred[offset + 3], pred[offset + 4]);
        let confidence = conf.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);

        steps.push(CopilotStepPrediction {
            value,
            tangent_in,
            tangent_out,
            confidence,
        });
    }

    crate::log!(
        "CurveCopilot raw output: {} steps, pred_len={}, conf_len={}, query_times={:?}",
        steps.len(),
        pred.len(),
        conf.len(),
        query_times
    );

    Ok(InferenceResultKind::CurveCopilotPredict { steps })
}
