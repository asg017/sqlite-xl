use thiserror::Error;

#[derive(Debug, PartialEq)]
pub struct CellReference {
    pub location: (u32, u32),
}

#[derive(Debug, PartialEq)]
pub struct RangeReference {
    pub start: (u32, u32),
    pub end: (u32, u32),
}

enum Reference {
    Cell(CellReference),
    Range((usize, usize)),
}

#[derive(Error, Debug, PartialEq)]
pub enum ParseCellReferenceError<'a> {
    #[error("the data for key `{0}` is not available")]
    Redaction(String),
    #[error("idk")]
    Unknown,
    #[error("Empty input")]
    Empty,

    #[error("Syntax error: {0}")]
    Syntax(&'a str),
}

pub fn column_name_to_idx(column_name: &str) -> Result<u32, ()> {
    let mut value: u32 = 0;
    let mut multiplier = 1;
    for c in column_name.chars().rev() {
        if c.is_ascii_lowercase() {
            let v: u32 = c as u32 - 'a' as u32 + 1;
            value += v * multiplier;
        } else if c.is_ascii_uppercase() {
            let v: u32 = c as u32 - 'A' as u32 + 1;
            value += v * multiplier;
        } else {
            todo!();
        }
        multiplier *= 26;
    }
    Ok(value  - 1)
}

fn cell_location(column: &str, row: u32) -> Result<(u32, u32), ()> {
    Ok((column_name_to_idx(column)?, row - 1))
}

pub fn parse_cell_reference(input: &str) -> Result<CellReference, ParseCellReferenceError> {
    let tokens = tokenize(input);
    let mut tokens = tokens.iter();
    let first = tokens.next().ok_or(ParseCellReferenceError::Empty)?;
    let second = tokens
        .next()
        .ok_or(ParseCellReferenceError::Syntax("expected row value"))?;
    let column = match first {
        Token::Identifier(id) => id,
        _ => return Err(ParseCellReferenceError::Syntax("asdf")),
    };
    let row = match second {
        Token::Number(n) => n,
        _ => {
            return Err(ParseCellReferenceError::Syntax(
                "expected a number for row value",
            ))
        }
    };

    Ok(CellReference {
        location: cell_location(column, *row).map_err(|e| ParseCellReferenceError::Unknown)?,
    })
}

#[derive(Error, Debug, PartialEq)]
pub enum ParseRangeReferenceError<'a> {
    #[error("idk")]
    Unknown,
    #[error("Empty input")]
    Empty,
    #[error("Syntax error: {0}")]
    Syntax(&'a str),
}

pub fn parse_range_reference(input: &str) -> Result<RangeReference, ParseRangeReferenceError> {
    let tokens = tokenize(input);
    let mut tokens = tokens.iter();
    let first = tokens.next().ok_or(ParseRangeReferenceError::Empty)?;
    let second = tokens
        .next()
        .ok_or(ParseRangeReferenceError::Syntax("expected row value"))?;

    let c1 = match first {
        Token::Identifier(id) => id,
        _ => return Err(ParseRangeReferenceError::Syntax("asdf")),
    };
    let r1 = match second {
        Token::Number(n) => n,
        _ => {
            return Err(ParseRangeReferenceError::Syntax(
                "expected a number for row value",
            ))
        }
    };
    let third = tokens.next().ok_or(ParseRangeReferenceError::Empty)?;
    if !matches!(third, Token::RangeOperator) {
        return Err(ParseRangeReferenceError::Syntax("asdf"));
    }
    let fourth = tokens.next().ok_or(ParseRangeReferenceError::Empty)?;
    let fifth = tokens
        .next()
        .ok_or(ParseRangeReferenceError::Syntax("expected row value"))?;

    let c2 = match fourth {
        Token::Identifier(id) => id,
        _ => return Err(ParseRangeReferenceError::Syntax("asdf")),
    };
    let r2 = match fifth {
        Token::Number(n) => n,
        _ => {
            return Err(ParseRangeReferenceError::Syntax(
                "expected a number for row value",
            ))
        }
    };

    Ok(RangeReference {
        start: cell_location(c1, *r1).map_err(|e| ParseRangeReferenceError::Unknown)?,
        end: cell_location(c2, *r2).map_err(|e| ParseRangeReferenceError::Unknown)?,
    })
}

#[derive(Debug, PartialEq)]
enum Token {
    Identifier(String),
    Number(u32),
    // ':'
    RangeOperator,
}

// "A1" => ["A", "1"]
// "AA123" => ["AA", "123"]
// "A1:B2" => ["A", "1", ":", "B", "2"]
// "A:A" => ["A", ":", "A"]
// "Sheet1!A:A" => ["Sheet1", "!", "A", ":", "A"]
fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_alphabetic() {
            let mut value = String::new();
            value.push(c);
            while let Some(c) = chars.peek() {
                if c.is_alphabetic() {
                    value.push(*c);
                    chars.next();
                    continue;
                }
                break;
            }
            tokens.push(Token::Identifier(value));
        } else if c.is_numeric() {
            let mut value = String::new();
            value.push(c);
            while let Some(c) = chars.peek() {
                if c.is_numeric() {
                    value.push(*c);
                    chars.next();
                    continue;
                }
                break;
            }
            tokens.push(Token::Number(value.parse::<u32>().unwrap()));
        } else if c == ':' {
            tokens.push(Token::RangeOperator);
        }
    }

    tokens
}

#[cfg(test)]
mod tests {

    use calamine::Range;

    use super::*;

    #[test]
    fn tokenizer() {
        assert_eq!(
            tokenize("A1"),
            vec![Token::Identifier("A".to_string()), Token::Number(1)]
        );
        assert_eq!(
            tokenize("AB123"),
            vec![Token::Identifier("AB".to_string()), Token::Number(123)]
        );
        assert_eq!(
            tokenize("A1:B3"),
            vec![
                Token::Identifier("A".to_string()),
                Token::Number(1),
                Token::RangeOperator,
                Token::Identifier("B".to_string()),
                Token::Number(3)
            ]
        );
    }

    #[test]
    fn test_parse_cell_reference() {
        assert_eq!(
            parse_cell_reference(""),
            Err(ParseCellReferenceError::Empty)
        );
        assert_eq!(
            parse_cell_reference("."),
            Err(ParseCellReferenceError::Empty)
        );
        assert_eq!(
            parse_cell_reference("A1"),
            Ok(CellReference { location: (0, 0) })
        );
        assert_eq!(
            parse_cell_reference("B2"),
            Ok(CellReference { location: (1, 1) })
        );
        assert_eq!(
            parse_cell_reference("J10"),
            Ok(CellReference { location: (9, 9) })
        );
        assert_eq!(
            parse_cell_reference("AA100"),
            Ok(CellReference { location: (26, 99) })
        );
        assert_eq!(
            parse_cell_reference("Ab1"),
            Ok(CellReference { location: (27, 0) })
        );
        assert_eq!(
            parse_cell_reference("zfD99999"),
            Ok(CellReference {
                location: (17735, 99998)
            })
        );
    }

    #[test]
    fn test_parse_range_reference() {
        assert_eq!(
            parse_range_reference("A1:B5"),
            Ok(RangeReference {
                start: (0, 0),
                end: (1, 4)
            })
        );
    }

    #[test]
    fn test_column_parsing() {
      assert_eq!(cell_location("A", 1), Ok((0, 0)));
      assert_eq!(cell_location("Z", 100), Ok((25, 99)));
      assert_eq!(column_name_to_idx("A"), Ok(0));
      assert_eq!(column_name_to_idx("a"), Ok(0));
      assert_eq!(column_name_to_idx("Z"), Ok(25));
      assert_eq!(column_name_to_idx("z"), Ok(25));
      assert_eq!(column_name_to_idx("AA"), Ok(26));
      assert_eq!(column_name_to_idx("ZFD"), Ok(17735));
    }
}
