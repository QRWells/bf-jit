use crate::error::{CompileError, CompileErrorKind};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BrainfuckIR {
    AddVal(u8),  // +
    SubVal(u8),  // -
    AddPtr(u32), // >
    SubPtr(u32), // <
    PutByte,     // .
    GetByte,     // ,
    Jz,          // [
    Jnz,         // ]
}

pub fn compile(source: &str) -> Result<Vec<BrainfuckIR>, CompileError> {
    let mut code = vec![];
    let mut stack = vec![];
    let mut line = 1;
    let mut col = 1;
    let source = source.chars().collect::<Vec<_>>();
    let len = source.len();
    let mut offset = 0;

    macro_rules! opt {
        ($ir: ident, $ch: literal) => {
            let mut val = 0;
            let mut j = offset;
            while j < len {
                if source[j] == $ch {
                    val += 1;
                    j += 1;
                } else {
                    break;
                }
            }
            offset = j - 1;
            code.push($ir(val));
        };
    }

    use BrainfuckIR::*;

    loop {
        match source[offset] {
            '\n' => {
                line += 1;
                col = 0;
            }
            '+' => {
                opt!(AddVal, '+');
            }
            '-' => {
                opt!(SubVal, '-');
            }
            '>' => {
                opt!(AddPtr, '>');
            }
            '<' => {
                opt!(SubPtr, '<');
            }
            ',' => code.push(BrainfuckIR::GetByte),
            '.' => code.push(BrainfuckIR::PutByte),
            '[' => {
                let pos = code.len();
                stack.push((pos, line, col));
                code.push(BrainfuckIR::Jz)
            }
            ']' => {
                stack.pop().ok_or(CompileError {
                    line,
                    col,
                    kind: CompileErrorKind::UnexpectedRightBracket,
                })?;

                code.push(BrainfuckIR::Jnz)
            }
            ' ' => {}
            _ => {
                return Err(CompileError {
                    line,
                    col,
                    kind: CompileErrorKind::UnclosedCharacter,
                });
            }
        }

        offset += 1;
        col += 1;

        if offset == source.len() {
            break;
        }
    }

    if let Some((_, line, col)) = stack.pop() {
        Err(CompileError {
            line,
            col,
            kind: CompileErrorKind::UnclosedLeftBracket,
        })
    } else {
        Ok(code)
    }
}

#[test]
fn compile_test() {
    let code = compile("+[,.]");
    assert_eq!(
        code.unwrap(),
        vec![
            BrainfuckIR::AddVal(1),
            BrainfuckIR::Jz,
            BrainfuckIR::GetByte,
            BrainfuckIR::PutByte,
            BrainfuckIR::Jnz,
        ]
    );

    let code = compile("[[]");
    assert_eq!(
        code.unwrap_err().kind,
        CompileErrorKind::UnclosedLeftBracket
    );

    let code = compile("[]]");
    assert_eq!(
        code.unwrap_err().kind,
        CompileErrorKind::UnexpectedRightBracket
    );
}

#[test]
fn optimize_test() {
    let code = compile("++++++++++----->><<");
    assert!(code.is_ok());
    let code = code.unwrap();
    assert_eq!(
        code,
        vec![
            BrainfuckIR::AddVal(10),
            BrainfuckIR::SubVal(5),
            BrainfuckIR::AddPtr(2),
            BrainfuckIR::SubPtr(2)
        ]
    );
}
