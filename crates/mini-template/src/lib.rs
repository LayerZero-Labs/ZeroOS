//! `mini-template`: a tiny, auditable template renderer intended for build-time scripts.
//!
//! Supported syntax (Jinja-like subset):
//! - `{% if <ident> %} ... {% else %} ... {% endif %}`
//!
//! Only boolean identifiers are supported; no expressions, no filters, no loops.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Context {
    bools: BTreeMap<String, bool>,
    strs: BTreeMap<String, String>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_bool(&mut self, name: impl Into<String>, value: bool) {
        self.bools.insert(name.into(), value);
    }

    pub fn with_bool(mut self, name: impl Into<String>, value: bool) -> Self {
        self.insert_bool(name, value);
        self
    }

    pub fn insert_str(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.strs.insert(name.into(), value.into());
    }

    pub fn with_str(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.insert_str(name, value);
        self
    }

    fn get_bool(&self, name: &str) -> Option<bool> {
        self.bools.get(name).copied()
    }

    fn get_str(&self, name: &str) -> Option<&str> {
        self.strs.get(name).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct RenderError {
    pub message: String,
    pub byte_offset: usize,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at byte {})", self.message, self.byte_offset)
    }
}

impl std::error::Error for RenderError {}

#[derive(Debug)]
struct Frame {
    cond_true: bool,
    in_else: bool,
}

fn should_emit(stack: &[Frame]) -> bool {
    // Emit only if every active frame selects this branch.
    stack
        .iter()
        .all(|f| if f.in_else { !f.cond_true } else { f.cond_true })
}

/// Render `template` using `ctx`.
pub fn render(template: &str, ctx: &Context) -> Result<String, RenderError> {
    let mut out = String::with_capacity(template.len());
    let mut stack: Vec<Frame> = Vec::new();

    let mut i = 0;
    while i < template.len() {
        let rest = &template[i..];
        let next_ctrl = rest.find("{%");
        let next_expr = rest.find("{{");
        let open = match (next_ctrl, next_expr) {
            (None, None) => None,
            (Some(a), None) => Some((a, true)),
            (None, Some(b)) => Some((b, false)),
            (Some(a), Some(b)) => Some(if a <= b { (a, true) } else { (b, false) }),
        };

        if let Some((open, is_ctrl)) = open {
            let text = &rest[..open];
            if should_emit(&stack) {
                out.push_str(text);
            }
            i += open;

            let rest2 = &template[i..];
            if is_ctrl {
                let close = rest2.find("%}").ok_or_else(|| RenderError {
                    message: "Unclosed template tag".to_string(),
                    byte_offset: i,
                })?;

                let tag = rest2[2..close].trim();
                let tag_offset = i;
                i += close + 2;

                if tag == "else" {
                    let top = stack.last_mut().ok_or_else(|| RenderError {
                        message: "{% else %} without matching {% if ... %}".to_string(),
                        byte_offset: tag_offset,
                    })?;
                    if top.in_else {
                        return Err(RenderError {
                            message: "Duplicate {% else %} in the same {% if %} block".to_string(),
                            byte_offset: tag_offset,
                        });
                    }
                    top.in_else = true;
                    continue;
                }

                if tag == "endif" {
                    if stack.pop().is_none() {
                        return Err(RenderError {
                            message: "{% endif %} without matching {% if ... %}".to_string(),
                            byte_offset: tag_offset,
                        });
                    }
                    continue;
                }

                if let Some(cond) = tag.strip_prefix("if ") {
                    let ident = cond.trim();
                    if ident.is_empty() {
                        return Err(RenderError {
                            message: "Empty identifier in {% if %}".to_string(),
                            byte_offset: tag_offset,
                        });
                    }
                    let cond_true = ctx.get_bool(ident).ok_or_else(|| RenderError {
                        message: format!("Unknown boolean identifier in template: {}", ident),
                        byte_offset: tag_offset,
                    })?;

                    stack.push(Frame {
                        cond_true,
                        in_else: false,
                    });
                    continue;
                }

                return Err(RenderError {
                    message: format!("Unknown template tag: {{% {} %}}", tag),
                    byte_offset: tag_offset,
                });
            } else {
                let close = rest2.find("}}").ok_or_else(|| RenderError {
                    message: "Unclosed template expression".to_string(),
                    byte_offset: i,
                })?;
                let expr = rest2[2..close].trim();
                let expr_offset = i;
                i += close + 2;

                if should_emit(&stack) {
                    let ident = expr;
                    if ident.is_empty() {
                        return Err(RenderError {
                            message: "Empty identifier in {{ ... }}".to_string(),
                            byte_offset: expr_offset,
                        });
                    }
                    let val = ctx.get_str(ident).ok_or_else(|| RenderError {
                        message: format!("Unknown string identifier in template: {}", ident),
                        byte_offset: expr_offset,
                    })?;
                    out.push_str(val);
                }
                continue;
            }
        } else {
            if should_emit(&stack) {
                out.push_str(rest);
            }
            break;
        }
    }

    if !stack.is_empty() {
        return Err(RenderError {
            message: "Unclosed {% if %} block(s)".to_string(),
            byte_offset: template.len(),
        });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn if_true_emits_then_branch() {
        let ctx = Context::new().with_bool("backtrace", true);
        let s = "a{% if backtrace %}b{% else %}c{% endif %}d";
        assert_eq!(render(s, &ctx).unwrap(), "abd");
    }

    #[test]
    fn if_false_emits_else_branch() {
        let ctx = Context::new().with_bool("backtrace", false);
        let s = "a{% if backtrace %}b{% else %}c{% endif %}d";
        assert_eq!(render(s, &ctx).unwrap(), "acd");
    }

    #[test]
    fn if_without_else() {
        let ctx = Context::new().with_bool("x", false);
        let s = "a{% if x %}b{% endif %}c";
        assert_eq!(render(s, &ctx).unwrap(), "ac");
    }

    #[test]
    fn nesting_works() {
        let ctx = Context::new().with_bool("a", true).with_bool("b", false);
        let s = "{% if a %}A{% if b %}B{% else %}C{% endif %}D{% endif %}";
        assert_eq!(render(s, &ctx).unwrap(), "ACD");
    }

    #[test]
    fn unknown_identifier_errors() {
        let ctx = Context::new();
        let err = render("{% if nope %}x{% endif %}", &ctx).unwrap_err();
        assert!(err.message.contains("Unknown boolean identifier"));
    }

    #[test]
    fn string_interpolation() {
        let ctx = Context::new().with_str("MEMORY_ORIGIN", "0x80000000");
        let s = "ORIGIN={{ MEMORY_ORIGIN }}";
        assert_eq!(render(s, &ctx).unwrap(), "ORIGIN=0x80000000");
    }
}
