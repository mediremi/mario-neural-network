struct NeuralNetwork {}

impl NeuralNetwork {
    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }
}
