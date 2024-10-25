mod payload;

use payload::Token;

#[wasm_bindgen::prelude::wasm_bindgen]
impl Token {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
}
