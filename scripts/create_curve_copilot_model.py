import numpy as np
import onnx
from onnx import TensorProto, helper


def main():
    rng = np.random.default_rng(123)

    context_input = helper.make_tensor_value_info(
        "context_keyframes", TensorProto.FLOAT, [1, 8, 6]
    )
    property_type_input = helper.make_tensor_value_info(
        "property_type", TensorProto.INT64, [1]
    )
    joint_category_input = helper.make_tensor_value_info(
        "joint_category", TensorProto.INT64, [1]
    )
    query_time_input = helper.make_tensor_value_info(
        "query_time", TensorProto.FLOAT, [1]
    )

    prediction_output = helper.make_tensor_value_info(
        "prediction", TensorProto.FLOAT, [1, 6]
    )
    confidence_output = helper.make_tensor_value_info(
        "confidence", TensorProto.FLOAT, [1, 1]
    )

    context_shape = helper.make_tensor("context_reshape_shape", TensorProto.INT64, [2], [1, 48])
    fc1_weight = rng.standard_normal((32, 48)).astype(np.float32) * 0.1
    fc1_bias = np.zeros(32, dtype=np.float32)
    pred_weight = rng.standard_normal((6, 32)).astype(np.float32) * 0.1
    pred_bias = np.zeros(6, dtype=np.float32)
    conf_weight = rng.standard_normal((1, 32)).astype(np.float32) * 0.1
    conf_bias = np.array([0.5], dtype=np.float32)

    initializers = [
        context_shape,
        helper.make_tensor("fc1_weight", TensorProto.FLOAT, [32, 48], fc1_weight.flatten()),
        helper.make_tensor("fc1_bias", TensorProto.FLOAT, [32], fc1_bias),
        helper.make_tensor("pred_weight", TensorProto.FLOAT, [6, 32], pred_weight.flatten()),
        helper.make_tensor("pred_bias", TensorProto.FLOAT, [6], pred_bias),
        helper.make_tensor("conf_weight", TensorProto.FLOAT, [1, 32], conf_weight.flatten()),
        helper.make_tensor("conf_bias", TensorProto.FLOAT, [1], conf_bias),
    ]

    reshape_node = helper.make_node(
        "Reshape", ["context_keyframes", "context_reshape_shape"], ["context_flat"]
    )
    fc1_node = helper.make_node(
        "Gemm", ["context_flat", "fc1_weight", "fc1_bias"], ["fc1_out"], transB=1
    )
    relu_node = helper.make_node("Relu", ["fc1_out"], ["relu_out"])
    pred_node = helper.make_node(
        "Gemm", ["relu_out", "pred_weight", "pred_bias"], ["prediction"], transB=1
    )
    conf_node = helper.make_node(
        "Gemm", ["relu_out", "conf_weight", "conf_bias"], ["conf_raw"], transB=1
    )
    sigmoid_node = helper.make_node("Sigmoid", ["conf_raw"], ["confidence"])

    graph = helper.make_graph(
        [reshape_node, fc1_node, relu_node, pred_node, conf_node, sigmoid_node],
        "curve_copilot",
        [context_input, property_type_input, joint_category_input, query_time_input],
        [prediction_output, confidence_output],
        initializer=initializers,
    )

    model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", 17)])
    model.ir_version = 8
    onnx.checker.check_model(model)
    onnx.save(model, "assets/ml/curve_copilot_dummy.onnx")
    print("Created assets/ml/curve_copilot_dummy.onnx (4 inputs, 2 outputs)")


if __name__ == "__main__":
    main()
