use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedBlock {
    #[serde(rename = "blockName")]
    pub block_name: Option<String>,
    pub attrs: Option<serde_json::Value>,
    #[serde(rename = "innerBlocks")]
    pub inner_blocks: Vec<ParsedBlock>,
    #[serde(rename = "innerHTML")]
    pub inner_html: String,
    #[serde(rename = "innerContent")]
    pub inner_content: Vec<Option<String>>,
}

impl ParsedBlock {
    pub fn new(
        block_name: Option<String>,
        attrs: Option<serde_json::Value>,
        inner_blocks: Vec<ParsedBlock>,
        inner_html: String,
        inner_content: Vec<Option<String>>,
    ) -> Self {
        ParsedBlock {
            block_name,
            attrs,
            inner_blocks,
            inner_html,
            inner_content,
        }
    }

    pub fn freeform(inner_html: String) -> Self {
        let map = serde_json::Map::new();
        ParsedBlock::new(
            None,
            Some(serde_json::Value::Object(map)),
            Vec::new(),
            inner_html.clone(),
            vec![Some(inner_html)],
        )
    }

    /// Recursively counts the total number of blocks (including this block and all nested `inner_blocks`).
    pub fn total_block_count(&self) -> usize {
        1 + self.inner_blocks.iter().map(Self::total_block_count).sum::<usize>()
    }

    /// Computes the maximum nesting depth of this block subtree (1 for a leaf block).
    pub fn max_nesting_depth(&self) -> usize {
        1 + self.inner_blocks.iter().map(Self::max_nesting_depth).max().unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum TokenType {
    NoMoreTokens,
    VoidBlock,
    BlockOpener,
    BlockCloser,
}

struct Token {
    token_type: TokenType,
    block_name: String,
    attrs: Option<serde_json::Value>,
    start_offset: usize,
    token_length: usize,
}

struct ParsedFrame {
    block: ParsedBlock,
    token_start: usize,
    token_length: usize,
    prev_offset: usize,
    leading_html_start: Option<usize>,
}

pub struct BlockParser<'a> {
    document: &'a str,
    offset: usize,
    output: Vec<ParsedBlock>,
    stack: Vec<ParsedFrame>,
}

impl<'a> BlockParser<'a> {
    pub fn new(document: &'a str) -> Self {
        BlockParser {
            document,
            offset: 0,
            output: Vec::new(),
            stack: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Vec<ParsedBlock> {
        self.offset = 0;
        self.output.clear();
        self.stack.clear();

        while self.proceed() {}

        // Take output
        std::mem::take(&mut self.output)
    }

    fn proceed(&mut self) -> bool {
        let stack_depth = self.stack.len();
        let token = self.next_token();
        let start_offset = token.start_offset;
        let token_length = token.token_length;

        // We may have some HTML soup before the next block.
        let leading_html_start = if start_offset > self.offset {
            Some(self.offset)
        } else {
            None
        };

        match token.token_type {
            TokenType::NoMoreTokens => {
                if stack_depth == 0 {
                    self.add_freeform(None);
                    return false;
                }

                if stack_depth == 1 {
                    self.add_block_from_stack(None);
                    return false;
                }

                while !self.stack.is_empty() {
                    self.add_block_from_stack(None);
                }
                false
            }
            TokenType::VoidBlock => {
                if stack_depth == 0 {
                    if let Some(lhs) = leading_html_start {
                        let html = &self.document[lhs..start_offset];
                        self.output.push(ParsedBlock::freeform(html.to_string()));
                    }
                    self.output.push(ParsedBlock::new(
                        Some(token.block_name),
                        token.attrs,
                        Vec::new(),
                        String::new(),
                        Vec::new(),
                    ));
                    self.offset = start_offset + token_length;
                    return true;
                }

                self.add_inner_block(
                    ParsedBlock::new(
                        Some(token.block_name),
                        token.attrs,
                        Vec::new(),
                        String::new(),
                        Vec::new(),
                    ),
                    start_offset,
                    token_length,
                    None,
                );
                self.offset = start_offset + token_length;
                true
            }
            TokenType::BlockOpener => {
                self.stack.push(ParsedFrame {
                    block: ParsedBlock::new(
                        Some(token.block_name),
                        token.attrs,
                        Vec::new(),
                        String::new(),
                        Vec::new(),
                    ),
                    token_start: start_offset,
                    token_length,
                    prev_offset: start_offset + token_length,
                    leading_html_start,
                });
                self.offset = start_offset + token_length;
                true
            }
            TokenType::BlockCloser => {
                if stack_depth == 0 {
                    self.add_freeform(None);
                    return false;
                }

                if stack_depth == 1 {
                    self.add_block_from_stack(Some(start_offset));
                    self.offset = start_offset + token_length;
                    return true;
                }

                let stack_top = self.stack.pop().expect("stack should not be empty");
                let html = &self.document[stack_top.prev_offset..start_offset];
                
                let mut block = stack_top.block;
                block.inner_html.push_str(html);
                block.inner_content.push(Some(html.to_string()));

                let frame_to_add = ParsedFrame {
                    block,
                    token_start: stack_top.token_start,
                    token_length: stack_top.token_length,
                    prev_offset: start_offset + token_length,
                    leading_html_start: stack_top.leading_html_start,
                };

                self.add_inner_block_frame(
                    frame_to_add,
                    Some(start_offset + token_length),
                );
                self.offset = start_offset + token_length;
                true
            }
        }
    }

    fn next_token(&mut self) -> Token {
        let mut current = self.offset;
        loop {
            if current + 4 > self.document.len() {
                return Token {
                    token_type: TokenType::NoMoreTokens,
                    block_name: String::new(),
                    attrs: None,
                    start_offset: self.document.len(),
                    token_length: 0,
                };
            }

            if &self.document[current..current + 4] == "<!--" {
                let inside = &self.document[current + 4..];
                let mut whitespace_len = 0;
                for c in inside.chars() {
                    if c.is_whitespace() {
                        whitespace_len += c.len_utf8();
                    } else {
                        break;
                    }
                }

                let start_after_ws = current + 4 + whitespace_len;
                if start_after_ws + 3 <= self.document.len() {
                    let remainder = &self.document[start_after_ws..];
                    let is_closer = remainder.starts_with('/');
                    let after_slash = if is_closer { &remainder[1..] } else { remainder };

                    if after_slash.starts_with("wp:") {
                        let block_name_start = start_after_ws + if is_closer { 1 } else { 0 } + 3;
                        let mut name_len = 0;
                        for c in self.document[block_name_start..].chars() {
                            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' || c == '/' {
                                name_len += c.len_utf8();
                            } else {
                                break;
                            }
                        }

                        let raw_block_name = &self.document[block_name_start..block_name_start + name_len];
                        let block_name = if raw_block_name.contains('/') {
                            raw_block_name.to_string()
                        } else {
                            format!("core/{}", raw_block_name)
                        };

                        let mut scan_pos = block_name_start + name_len;
                        while scan_pos < self.document.len() && self.document.as_bytes()[scan_pos].is_ascii_whitespace() {
                            scan_pos += 1;
                        }

                        let mut attrs = None;
                        let mut has_attrs = false;

                        if scan_pos < self.document.len() && self.document.as_bytes()[scan_pos] == b'{' {
                            let attrs_start = scan_pos;
                            let mut brace_count = 0;
                            let mut in_string = false;
                            let mut escaped = false;

                            while scan_pos < self.document.len() {
                                let b = self.document.as_bytes()[scan_pos];
                                if in_string {
                                    if escaped {
                                        escaped = false;
                                    } else if b == b'\\' {
                                        escaped = true;
                                    } else if b == b'"' {
                                        in_string = false;
                                    }
                                } else {
                                    if b == b'"' {
                                        in_string = true;
                                        escaped = false;
                                    } else if b == b'{' {
                                        brace_count += 1;
                                    } else if b == b'}' {
                                        brace_count -= 1;
                                        if brace_count == 0 {
                                            scan_pos += 1;
                                            has_attrs = true;
                                            break;
                                        }
                                    }
                                }
                                scan_pos += 1;
                            }

                            if has_attrs {
                                let attrs_str = &self.document[attrs_start..scan_pos];
                                attrs = match serde_json::from_str::<serde_json::Value>(attrs_str) {
                                    Ok(v) => Some(v),
                                    Err(_) => Some(serde_json::Value::Null),
                                };
                            }
                        }

                        while scan_pos < self.document.len() && self.document.as_bytes()[scan_pos].is_ascii_whitespace() {
                            scan_pos += 1;
                        }

                        let mut is_void = false;
                        if scan_pos + 4 <= self.document.len() && &self.document[scan_pos..scan_pos + 4] == "/-->" {
                            is_void = true;
                            scan_pos += 4;
                        } else if scan_pos + 3 <= self.document.len() && &self.document[scan_pos..scan_pos + 3] == "-->" {
                            scan_pos += 3;
                        } else {
                            if let Some(offset_to_end) = self.document[scan_pos..].find("-->") {
                                scan_pos += offset_to_end + 3;
                            } else {
                                scan_pos = self.document.len();
                            }
                        }

                        let token_type = if is_void {
                            TokenType::VoidBlock
                        } else if is_closer {
                            TokenType::BlockCloser
                        } else {
                            TokenType::BlockOpener
                        };

                        let token_length = scan_pos - current;

                        // Closers return null attributes in JS parser
                        let attrs_val = if token_type == TokenType::BlockCloser {
                            None
                        } else if attrs.is_none() {
                            Some(serde_json::Value::Object(serde_json::Map::new()))
                        } else {
                            attrs
                        };

                        return Token {
                            token_type,
                            block_name,
                            attrs: attrs_val,
                            start_offset: current,
                            token_length,
                        };
                    }
                }

                if let Some(offset_to_end) = self.document[current..].find("-->") {
                    current += offset_to_end + 3;
                } else {
                    current = self.document.len();
                }
            } else {
                current += 1;
            }
        }
    }

    fn add_freeform(&mut self, raw_length: Option<usize>) {
        let length = match raw_length {
            Some(l) => l,
            None => self.document.len() - self.offset,
        };
        if length == 0 {
            return;
        }
        let html = &self.document[self.offset..self.offset + length];
        self.output.push(ParsedBlock::freeform(html.to_string()));
    }

    fn add_inner_block(
        &mut self,
        block: ParsedBlock,
        token_start: usize,
        token_length: usize,
        last_offset: Option<usize>,
    ) {
        let parent_idx = self.stack.len() - 1;
        let parent = &mut self.stack[parent_idx];

        let prev_offset = parent.prev_offset;
        let html = &self.document[prev_offset..token_start];

        if !html.is_empty() {
            parent.block.inner_html.push_str(html);
            parent.block.inner_content.push(Some(html.to_string()));
        }

        parent.block.inner_blocks.push(block);
        parent.block.inner_content.push(None);

        parent.prev_offset = match last_offset {
            Some(o) => o,
            None => token_start + token_length,
        };
    }

    fn add_inner_block_frame(
        &mut self,
        frame: ParsedFrame,
        last_offset: Option<usize>,
    ) {
        let parent_idx = self.stack.len() - 1;
        let parent = &mut self.stack[parent_idx];

        let prev_offset = parent.prev_offset;
        let html = &self.document[prev_offset..frame.token_start];

        if !html.is_empty() {
            parent.block.inner_html.push_str(html);
            parent.block.inner_content.push(Some(html.to_string()));
        }

        parent.block.inner_blocks.push(frame.block);
        parent.block.inner_content.push(None);

        parent.prev_offset = match last_offset {
            Some(o) => o,
            None => frame.token_start + frame.token_length,
        };
    }

    fn add_block_from_stack(&mut self, end_offset: Option<usize>) {
        let mut frame = self.stack.pop().expect("stack should not be empty");

        let html = match end_offset {
            Some(e) => &self.document[frame.prev_offset..e],
            None => &self.document[frame.prev_offset..],
        };

        if !html.is_empty() {
            frame.block.inner_html.push_str(html);
            frame.block.inner_content.push(Some(html.to_string()));
        }

        if let Some(leading_html_start) = frame.leading_html_start {
            let leading_html = &self.document[leading_html_start..frame.token_start];
            self.output.push(ParsedBlock::freeform(leading_html.to_string()));
        }

        self.output.push(frame.block);
    }
}

pub fn parse(doc: &str) -> Vec<ParsedBlock> {
    let mut parser = BlockParser::new(doc);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nested_blocks_and_metrics() {
        let doc = "<!-- wp:group {\"layout\":{\"type\":\"constrained\"}} -->\n<div class=\"wp-block-group\"><!-- wp:paragraph {\"align\":\"center\"} -->\n<p class=\"has-text-align-center\">Hello Gutenberg</p>\n<!-- /wp:paragraph --></div>\n<!-- /wp:group -->";
        let blocks = parse(doc);
        assert_eq!(blocks.len(), 1);
        let root = &blocks[0];
        assert_eq!(root.block_name.as_deref(), Some("core/group"));
        assert_eq!(root.total_block_count(), 2);
        assert_eq!(root.max_nesting_depth(), 2);
    }
}
