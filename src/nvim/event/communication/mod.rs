mod image;
mod resource;
mod resource_link;
mod text;

use agent_client_protocol::{ContentBlock, ContentChunk, Error as AcpError, Result};
use nvim_oxi::Dictionary;

pub fn communication(content: ContentBlock) -> Result<(Dictionary, String)> {
    match content {
        ContentBlock::Resource(block) => resource::resource_event(block),
        ContentBlock::ResourceLink(block) => resource_link::resource_link_event(block),
        ContentBlock::Image(image) => image::image_event(image),
        ContentBlock::Text(text) => text::text_event(text),
        ContentBlock::Audio(_) => Err(AcpError::method_not_found()),
        _ => Err(AcpError::method_not_found()),
    }
}
