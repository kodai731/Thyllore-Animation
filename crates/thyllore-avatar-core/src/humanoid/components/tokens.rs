use crate::expression::components::side::Side;

#[derive(Clone, Debug, Default)]
pub struct BoneNameTokens {
    pub tokens: Vec<String>,
    pub side: Option<Side>,
}
