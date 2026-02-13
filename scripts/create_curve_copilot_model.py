import numpy as np
import onnx
from onnx import TensorProto, helper


def main():
    rng = np.random.default_rng(123)
    fc1_weight = rng.standard_normal((32, 20)).astype(np.float32) * 0.1
    fc1_bias = np.zeros(32, dtype=np.float32)
    fc2_weight = rng.standard_normal((21, 32)).astype(np.float32) * 0.1
    fc2_bias = np.zeros(21, dtype=np.float32)

    initializers = [
        helper.make_tensor("fc1_weight", TensorProto.FLOAT, [32, 20], fc1_weight.flatten()),
        helper.make_tensor("fc1_bias", TensorProto.FLOAT, [32], fc1_bias),
        helper.make_tensor("fc2_weight", TensorProto.FLOAT, [21, 32], fc2_weight.flatten()),
        helper.make_tensor("fc2_bias", TensorProto.FLOAT, [21], fc2_bias),
    ]

    input_tensor = helper.make_tensor_value_info("input", TensorProto.FLOAT, ["batch", 20])
    output_tensor = helper.make_tensor_value_info("output", TensorProto.FLOAT, ["batch", 21])

    fc1_node = helper.make_node("Gemm", ["input", "fc1_weight", "fc1_bias"], ["fc1_out"], transB=1)
    relu_node = helper.make_node("Relu", ["fc1_out"], ["relu_out"])
    fc2_node = helper.make_node("Gemm", ["relu_out", "fc2_weight", "fc2_bias"], ["output"], transB=1)

    graph = helper.make_graph(
        [fc1_node, relu_node, fc2_node],
        "curve_copilot",
        [input_tensor],
        [output_tensor],
        initializer=initializers,
    )

    model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", 17)])
    model.ir_version = 8
    onnx.checker.check_model(model)
    onnx.save(model, "assets/ml/curve_copilot_dummy.onnx")
    print("Created assets/ml/curve_copilot_dummy.onnx")


if __name__ == "__main__":
    main()
