use serde::{Deserialize, Serialize};

/// various programs and their ids, optionally including their idls
#[remain::sorted]
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Programs {
    /// address of the tulip lending program
    pub lending: Program,
    pub pyth: Program,
}

/// a deployed program we interact with
#[remain::sorted]
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Program {
    /// the program id
    pub id: String,
    /// path to the anchor generated idl file (if at all)
    pub idl_path: String,
}
