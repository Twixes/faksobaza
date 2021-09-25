use super::errors::*;
use super::tokenizer::*;

#[derive(Debug, PartialEq, Eq)]
pub struct ExpectOk<'t, O> {
    pub rest: &'t [Token],
    pub tokens_consumed_count: usize,
    pub outcome: O,
}
pub type ExpectResult<'t, O> = Result<ExpectOk<'t, O>, SyntaxError>;

// Generic expects

pub fn expect_token_value<'t>(
    tokens: &'t [Token],
    expected_token_value: &TokenValue,
) -> ExpectResult<'t, ()> {
    match tokens.first() {
        None
        | Some(Token {
            value: TokenValue::Delimiting(Delimiter::Semicolon),
            ..
        }) => Err(SyntaxError(format!(
            "Expected `{}`, instead found end of statement.",
            expected_token_value
        ))),
        Some(found_token) => {
            if &found_token.value == expected_token_value {
                Ok(ExpectOk {
                    rest: &tokens[1..],
                    tokens_consumed_count: 1,
                    outcome: (),
                })
            } else {
                Err(SyntaxError(format!(
                    "Expected `{}`, instead found {}.",
                    expected_token_value, found_token
                )))
            }
        }
    }
}

pub fn expect_token_values_sequence<'t>(
    tokens: &'t [Token],
    expected_token_values: &[TokenValue],
) -> ExpectResult<'t, ()> {
    for (token_index, expected_token_value) in expected_token_values.iter().enumerate() {
        expect_token_value(&tokens[token_index..], expected_token_value)?;
    }
    let tokens_consumed_count = expected_token_values.len();
    Ok(ExpectOk {
        rest: &tokens[tokens_consumed_count..],
        tokens_consumed_count,
        outcome: (),
    })
}

pub fn expect_identifier<'t>(tokens: &'t [Token]) -> ExpectResult<'t, String> {
    match tokens.first() {
        None
        | Some(Token {
            value: TokenValue::Delimiting(Delimiter::Semicolon),
            ..
        }) => Err(SyntaxError(
            "Expected an identifier, instead found end of statement.".to_string(),
        )),
        Some(Token {
            value: TokenValue::Arbitrary(value),
            ..
        }) => Ok(ExpectOk {
            rest: &tokens[1..],
            tokens_consumed_count: 1,
            outcome: value.to_owned(),
        }),
        Some(wrong_token) => Err(SyntaxError(format!(
            "Expected an identifier, instead found {}.",
            wrong_token
        ))),
    }
}

pub fn expect_end_of_statement<'t>(tokens: &'t [Token]) -> ExpectResult<'t, ()> {
    match tokens.first() {
        None => Ok(ExpectOk {
            rest: tokens,
            tokens_consumed_count: 0,
            outcome: (),
        }),
        Some(Token {
            value: TokenValue::Delimiting(Delimiter::Semicolon),
            ..
        }) => {
            if tokens.len() > 1 {
                Err(SyntaxError("Found tokens after a semicolon! Only a single statement at once can be provided.".to_string()))
            } else {
                Ok(ExpectOk {
                    rest: &tokens[1..],
                    tokens_consumed_count: 1,
                    outcome: (),
                })
            }
        }
        Some(wrong_token) => Err(SyntaxError(format!(
            "Expected no more tokens or a semicolon, instead found {}.",
            wrong_token
        ))),
    }
}

pub fn expect_enclosed<'t, O>(
    tokens: &'t [Token],
    expect_inside: fn(&'t [Token]) -> ExpectResult<'t, O>,
) -> ExpectResult<'t, O> {
    let ExpectOk { rest, .. } = expect_token_value(
        tokens,
        &TokenValue::Delimiting(Delimiter::ParenthesisOpening),
    )?;
    let ExpectOk {
        rest,
        tokens_consumed_count,
        outcome,
    } = expect_inside(rest)?;
    let ExpectOk { rest, .. } =
        expect_token_value(rest, &TokenValue::Delimiting(Delimiter::ParenthesisClosing))?;
    let tokens_consumed_count = tokens_consumed_count + 2; // Account for parentheses
    Ok(ExpectOk {
        rest,
        tokens_consumed_count,
        outcome,
    })
}

pub fn expect_comma_separated<'t, O>(
    tokens: &'t [Token],
    expect_element: fn(&'t [Token]) -> ExpectResult<'t, O>,
) -> ExpectResult<'t, Vec<O>> {
    let mut tokens_consumed_total_count = 0;
    let mut outcomes = Vec::<O>::new();
    loop {
        // Parse next element
        let ExpectOk {
            tokens_consumed_count,
            outcome,
            ..
        } = expect_element(&tokens[tokens_consumed_total_count..])?;
        tokens_consumed_total_count += tokens_consumed_count;
        outcomes.push(outcome);
        // Check for the comma (trailing comma disallowed)
        match expect_token_value(
            &tokens[tokens_consumed_total_count..],
            &TokenValue::Delimiting(Delimiter::Comma),
        ) {
            Err(_) => break, // If there's no comma after this element, it's time to break out of the loop
            _ => {
                tokens_consumed_total_count += 1;
            }
        }
    }
    Ok(ExpectOk {
        rest: &tokens[tokens_consumed_total_count..],
        tokens_consumed_count: tokens_consumed_total_count,
        outcome: outcomes,
    })
}

// Semantic expects

#[derive(Debug, PartialEq, Eq)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataTypeWrapped,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TableDefinition {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
}

pub fn expect_data_type_wrapped<'t>(tokens: &'t [Token]) -> ExpectResult<'t, DataTypeWrapped> {
    let mut tokens_consumed_count = 0;
    let mut is_nullable = false;
    let data_type: DataType;
    let nullable_sequence = &[
        TokenValue::Const(Keyword::Nullable),
        TokenValue::Delimiting(Delimiter::ParenthesisOpening),
    ];
    match expect_token_values_sequence(tokens, nullable_sequence) {
        Ok(ExpectOk { outcome: (), .. }) => {
            tokens_consumed_count += nullable_sequence.len();
            is_nullable = true;
        }
        _ => (),
    };
    match tokens[tokens_consumed_count..].first() {
        None
        | Some(Token {
            value: TokenValue::Delimiting(Delimiter::Semicolon),
            ..
        }) => {
            return Err(SyntaxError(if is_nullable {
                "Expected a type, instead found end of statement.".to_string()
            } else {
                "Expected a type or `NULLABLE(`, instead found end of statement.".to_string()
            }))
        }
        Some(Token {
            value: TokenValue::Type(found_data_type),
            ..
        }) => {
            tokens_consumed_count += 1;
            data_type = *found_data_type;
        }
        Some(wrong_token) => {
            return Err(SyntaxError(if is_nullable {
                format!("Expected a type, instead found {}.", wrong_token)
            } else {
                format!(
                    "Expected a type or `NULLABLE(`, instead found {}.",
                    wrong_token
                )
            }))
        }
    };
    if is_nullable {
        match tokens[tokens_consumed_count..].first() {
            None
            | Some(Token {
                value: TokenValue::Delimiting(Delimiter::Semicolon),
                ..
            }) => {
                return Err(SyntaxError(
                    "Expected a closing parenthesis, instead found end of statement.".to_string(),
                ))
            }
            Some(Token {
                value: TokenValue::Delimiting(Delimiter::ParenthesisClosing),
                ..
            }) => Ok(ExpectOk {
                rest: &tokens[tokens_consumed_count + 1..],
                tokens_consumed_count: tokens_consumed_count + 1,
                outcome: DataTypeWrapped {
                    data_type,
                    is_nullable,
                },
            }),
            Some(wrong_token) => {
                return Err(SyntaxError(format!(
                    "Expected a closing parenthesis, instead found {}.",
                    wrong_token
                )))
            }
        }
    } else {
        Ok(ExpectOk {
            rest: &tokens[tokens_consumed_count..],
            tokens_consumed_count,
            outcome: DataTypeWrapped {
                data_type,
                is_nullable,
            },
        })
    }
}

pub fn expect_column_definition<'t>(tokens: &'t [Token]) -> ExpectResult<'t, ColumnDefinition> {
    let ExpectOk {
        rest,
        tokens_consumed_count: tokens_consumed_count_name,
        outcome: name,
    } = expect_identifier(tokens)?;
    let ExpectOk {
        rest,
        tokens_consumed_count: tokens_consumed_count_data_type,
        outcome: data_type,
    } = expect_data_type_wrapped(rest)?;
    Ok(ExpectOk {
        rest,
        tokens_consumed_count: tokens_consumed_count_name + tokens_consumed_count_data_type,
        outcome: ColumnDefinition { name, data_type },
    })
}

pub fn expect_table_definition<'t>(tokens: &'t [Token]) -> ExpectResult<'t, TableDefinition> {
    let ExpectOk {
        rest,
        tokens_consumed_count: tokens_consumed_count_name,
        outcome: name,
    } = expect_identifier(tokens)?;
    let ExpectOk {
        rest,
        tokens_consumed_count: tokens_consumed_count_columns,
        outcome: columns,
    } = expect_enclosed(rest, |tokens_enclosed| {
        Ok(expect_comma_separated(
            tokens_enclosed,
            expect_column_definition,
        )?)
    })?;
    Ok(ExpectOk {
        rest,
        tokens_consumed_count: tokens_consumed_count_name + tokens_consumed_count_columns,
        outcome: TableDefinition { name, columns },
    })
}

// Generic expect tests

#[cfg(test)]
mod expect_token_sequence_tests {
    use super::*;

    #[test]
    fn returns_ok() {
        assert_eq!(
            expect_token_values_sequence(
                &[
                    Token {
                        value: TokenValue::Const(Keyword::If),
                        line_number: 1
                    },
                    Token {
                        value: TokenValue::Const(Keyword::Not),
                        line_number: 1
                    },
                    Token {
                        value: TokenValue::Const(Keyword::Exists),
                        line_number: 1
                    }
                ],
                &[
                    TokenValue::Const(Keyword::If),
                    TokenValue::Const(Keyword::Not),
                    TokenValue::Const(Keyword::Exists),
                ]
            ),
            Ok(ExpectOk {
                rest: &[][..],
                tokens_consumed_count: 3,
                outcome: ()
            })
        )
    }

    #[test]
    fn returns_error_if_third_token_invalid() {
        assert_eq!(
            expect_token_values_sequence(
                &[
                    Token {
                        value: TokenValue::Const(Keyword::If),
                        line_number: 1
                    },
                    Token {
                        value: TokenValue::Const(Keyword::Not),
                        line_number: 1
                    },
                    Token {
                        value: TokenValue::Arbitrary("xyz".to_string()),
                        line_number: 1
                    }
                ],
                &[
                    TokenValue::Const(Keyword::If),
                    TokenValue::Const(Keyword::Not),
                    TokenValue::Const(Keyword::Exists),
                ]
            ),
            Err(SyntaxError(
                "Expected `EXISTS`, instead found `xyz` at line 1.".to_string()
            ))
        )
    }

    #[test]
    fn returns_error_if_too_few_tokens() {
        assert_eq!(
            expect_token_values_sequence(
                &[Token {
                    value: TokenValue::Const(Keyword::If),
                    line_number: 1
                }],
                &[
                    TokenValue::Const(Keyword::If),
                    TokenValue::Const(Keyword::Not),
                    TokenValue::Const(Keyword::Exists),
                ]
            ),
            Err(SyntaxError(
                "Expected `NOT`, instead found end of statement.".to_string()
            ))
        )
    }

    #[test]
    fn returns_error_if_eos() {
        assert_eq!(
            expect_token_values_sequence(
                &[],
                &[
                    TokenValue::Const(Keyword::If),
                    TokenValue::Const(Keyword::Not),
                    TokenValue::Const(Keyword::Exists),
                ]
            ),
            Err(SyntaxError(
                "Expected `IF`, instead found end of statement.".to_string()
            ))
        )
    }
}

#[cfg(test)]
mod expect_token_single_tests {
    use super::*;

    #[test]
    fn returns_ok() {
        assert_eq!(
            expect_token_value(
                &[
                    Token {
                        value: TokenValue::Const(Keyword::Primary),
                        line_number: 1
                    },
                    Token {
                        value: TokenValue::Arbitrary("foo".to_string()),
                        line_number: 1
                    }
                ],
                &TokenValue::Const(Keyword::Primary)
            ),
            Ok(ExpectOk {
                rest: &[Token {
                    value: TokenValue::Arbitrary("foo".to_string()),
                    line_number: 1
                }][..],
                tokens_consumed_count: 1,
                outcome: ()
            })
        )
    }

    #[test]
    fn returns_error_if_const_token() {
        assert_eq!(
            expect_token_value(
                &[Token {
                    value: TokenValue::Const(Keyword::Create),
                    line_number: 1
                }],
                &TokenValue::Const(Keyword::Primary)
            ),
            Err(SyntaxError(
                "Expected `PRIMARY`, instead found `CREATE` at line 1.".to_string()
            ))
        )
    }

    #[test]
    fn returns_error_if_eos() {
        assert_eq!(
            expect_token_value(&[], &TokenValue::Const(Keyword::Primary)),
            Err(SyntaxError(
                "Expected `PRIMARY`, instead found end of statement.".to_string()
            ))
        )
    }
}

#[cfg(test)]
mod expect_identifier_tests {
    use super::*;

    #[test]
    fn returns_ok() {
        assert_eq!(
            expect_identifier(&[Token {
                value: TokenValue::Arbitrary("foo".to_string()),
                line_number: 1
            }]),
            Ok(ExpectOk {
                rest: &[][..],
                tokens_consumed_count: 1,
                outcome: "foo".to_string()
            })
        )
    }

    #[test]
    fn returns_error_if_const_token() {
        assert_eq!(
            expect_identifier(&[Token {
                value: TokenValue::Const(Keyword::Create),
                line_number: 1
            }]),
            Err(SyntaxError(
                "Expected an identifier, instead found `CREATE` at line 1.".to_string()
            ))
        )
    }

    #[test]
    fn returns_error_if_eos() {
        assert_eq!(
            expect_identifier(&[]),
            Err(SyntaxError(
                "Expected an identifier, instead found end of statement.".to_string()
            ))
        )
    }
}

// Semantic expect tests

#[cfg(test)]
mod expect_data_type_wrapped_tests {
    use super::*;

    #[test]
    fn returns_ok_uint64() {
        assert_eq!(
            expect_data_type_wrapped(&[Token {
                value: TokenValue::Type(DataType::UInt64),
                line_number: 1
            }]),
            Ok(ExpectOk {
                rest: &[][..],
                tokens_consumed_count: 1,
                outcome: DataTypeWrapped {
                    data_type: DataType::UInt64,
                    is_nullable: false
                }
            })
        )
    }

    #[test]
    fn returns_ok_nullable_timestamp() {
        assert_eq!(
            expect_data_type_wrapped(&[
                Token {
                    value: TokenValue::Const(Keyword::Nullable),
                    line_number: 1
                },
                Token {
                    value: TokenValue::Delimiting(Delimiter::ParenthesisOpening),
                    line_number: 1
                },
                Token {
                    value: TokenValue::Type(DataType::Timestamp),
                    line_number: 1
                },
                Token {
                    value: TokenValue::Delimiting(Delimiter::ParenthesisClosing),
                    line_number: 1
                }
            ]),
            Ok(ExpectOk {
                rest: &[][..],
                tokens_consumed_count: 4,
                outcome: DataTypeWrapped {
                    data_type: DataType::Timestamp,
                    is_nullable: true
                }
            })
        )
    }

    #[test]
    fn returns_error_if_nullable_not_closed() {
        assert_eq!(
            expect_data_type_wrapped(&[
                Token {
                    value: TokenValue::Const(Keyword::Nullable),
                    line_number: 1
                },
                Token {
                    value: TokenValue::Delimiting(Delimiter::ParenthesisOpening),
                    line_number: 1
                },
                Token {
                    value: TokenValue::Type(DataType::Timestamp),
                    line_number: 1
                },
                Token {
                    value: TokenValue::Delimiting(Delimiter::Comma),
                    line_number: 1
                }
            ]),
            Err(SyntaxError(
                "Expected a closing parenthesis, instead found `,` at line 1.".to_string()
            ))
        )
    }

    #[test]
    fn returns_error_if_no_type() {
        assert_eq!(
            expect_data_type_wrapped(&[Token {
                value: TokenValue::Arbitrary("foo".to_string()),
                line_number: 1
            }]),
            Err(SyntaxError(
                "Expected a type or `NULLABLE(`, instead found `foo` at line 1.".to_string()
            ))
        )
    }

    #[test]
    fn returns_error_if_neos() {
        assert_eq!(
            expect_data_type_wrapped(&[]),
            Err(SyntaxError(
                "Expected a type or `NULLABLE(`, instead found end of statement.".to_string()
            ))
        )
    }

    #[test]
    fn returns_error_if_no_type_but_nullable() {
        assert_eq!(
            expect_data_type_wrapped(&[
                Token {
                    value: TokenValue::Const(Keyword::Nullable),
                    line_number: 1
                },
                Token {
                    value: TokenValue::Delimiting(Delimiter::ParenthesisOpening),
                    line_number: 1
                },
                Token {
                    value: TokenValue::Arbitrary("bar".to_string()),
                    line_number: 1
                }
            ]),
            Err(SyntaxError(
                "Expected a type, instead found `bar` at line 1.".to_string()
            ))
        )
    }

    #[test]
    fn returns_error_if_eos_but_nullable() {
        assert_eq!(
            expect_data_type_wrapped(&[
                Token {
                    value: TokenValue::Const(Keyword::Nullable),
                    line_number: 1
                },
                Token {
                    value: TokenValue::Delimiting(Delimiter::ParenthesisOpening),
                    line_number: 1
                }
            ]),
            Err(SyntaxError(
                "Expected a type, instead found end of statement.".to_string()
            ))
        )
    }
}
