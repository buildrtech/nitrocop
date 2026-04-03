pub mod block_single_line_braces;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(block_single_line_braces::BlockSingleLineBraces));
}
