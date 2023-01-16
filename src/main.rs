#![allow(dead_code)]
#![allow(unused_assignments)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(unused_imports)]
/*
A Tiny Scripting Language
---------
(C) Luminoso 2021 / All Rights Reserved
*/
use logos::Logos;
use std::collections::HashMap;
use std::fs;
use std::iter::Peekable;
use std::slice::SliceIndex;

//a bit of fancyness to make a bit below look neat ig
macro_rules! either {
    ($test:expr => $true_expr:expr; $false_expr:expr) => {
        if $test {
            $true_expr
        } else {
            $false_expr
        }
    };
}

fn parse(
    lex: &mut Peekable<std::slice::Iter<'_, ParsableToken>>,
    //current token
    stok: Token,
    //current string
    sstr: String,
    //prior token
    ptok: Token,
    //prior string
    pstr: String,
    //state
    pstate: &mut ParserState,
) -> ASTNode {
    if pstate.debug {
        println!(
            "\x1b[33m [Parse] Parse called w/ stok: {:?} sstr: {} ptok: {:?} pstr: {} pstate: {:?}\x1b[0m",
            stok, sstr, ptok, pstr, pstate
        );
    }
    match stok {
        /* Language Atoms [Number/Text/Bool/Identifier] */
        //numbers
        Token::Number | Token::DecimalNumber => {
            // decide if this number is "on it's own" or if it has an operator attached to it
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            match nx_tok {
                //we're just returning the number on it's lonesome
                Token::KwTerminator | Token::KwTo | Token::KwComma | Token::KwLBrace => {
                    return ASTNode::Number(sstr.parse::<f32>().unwrap());
                }
                Token::KwRBrace => {
                    pstate.encounteredRBrace = true;
                    return ASTNode::Number(sstr.parse::<f32>().unwrap());
                }
                Token::KwRParen => {
                    pstate.encounteredRParen = true;
                    return ASTNode::Number(sstr.parse::<f32>().unwrap());
                }
                //operators
                _ => {
                    //running parse on an operator will yield an operator AST node that is appropriate setup, provided we set the last token field correctly.
                    //actually forward so we're on the operator
                    lex.next();
                    //now do the call and return
                    return parse(lex, nx_tok, nx_str, stok, sstr, pstate);
                }
            }
        }
        //text - this is almost exactly like numbers so comments have been removed.
        Token::Text => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            //clean the quotes off the ends
            let retstr = sstr[1..sstr.len() - 1].to_string();
            match nx_tok {
                Token::KwTerminator | Token::KwTo | Token::KwComma | Token::KwLBrace => {
                    return ASTNode::Text(retstr.clone());
                }
                Token::KwRBrace => {
                    pstate.encounteredRBrace = true;
                    return ASTNode::Text(retstr.clone());
                }
                Token::KwRParen => {
                    pstate.encounteredRParen = true;
                    return ASTNode::Text(retstr.clone());
                }
                _ => {
                    lex.next();
                    return parse(lex, nx_tok, nx_str, stok, retstr, pstate);
                }
            }
        }
        //booleans - ditto w/ above
        Token::KwTrue | Token::KwFalse => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            match nx_tok {
                Token::KwTerminator | Token::KwTo | Token::KwComma | Token::KwLBrace => {
                    return either!(stok == Token::KwTrue => ASTNode::Boolean(true); ASTNode::Boolean(false));
                }
                Token::KwLBrace => {
                    pstate.encounteredRBrace = true;
                    return either!(stok == Token::KwTrue => ASTNode::Boolean(true); ASTNode::Boolean(false));
                }
                Token::KwRParen => {
                    pstate.encounteredRParen = true;
                    return either!(stok == Token::KwTrue => ASTNode::Boolean(true); ASTNode::Boolean(false));
                }
                _ => {
                    lex.next();
                    return parse(lex, nx_tok, nx_str, stok, sstr, pstate);
                }
            }
        }
        //the dreaded one - Identifiers
        Token::Identifier => {
            //first, we need to decide if this is a function call, or a variable.
            //to do this, we'll check against the registered function names.
            if pstate.registeredFnNames.contains(&sstr) {
                //it's a function invoke
                //the next token will be a LParen ( - we don't need this, so we skip it
                lex.next();
                //now, we read out parameters until such time as we encounter an RParen, which signifies the end of the parameter list
                //and that it's time to move on.
                let mut params: Vec<ASTNode> = vec![];
                loop {
                    let current_tokp = lex.next().unwrap();
                    let current_token = current_tokp.token.to_owned();
                    let current_str = current_tokp.text.to_owned();
                    if pstate.debug {
                        println!(
                            "\x1b[034m[Fn Call] Checking Potential Argument: {:?} {}\x1b[0m",
                            current_token, current_str
                        );
                    }
                    //if we encounter a RParen, or our parser state claims we have,we're done.
                    if current_token == Token::KwRParen {
                        if pstate.debug {
                            println!("\x1b[31mKwRParen -> break;\x1b[0m");
                        }
                        break;
                    } else if pstate.encounteredRParen {
                        if pstate.debug {
                            println!("\x1b[31mencountered KwRParen previously. -> break;\x1b[0m");
                        }
                        pstate.encounteredRParen = false;
                        break;
                    }
                    //recursively parse this argument
                    if pstate.debug {
                        println!(
                            "\x1b[36m[Fn Call] Parsing Argument: {:?} {}\x1b[0m",
                            current_token, current_str
                        );
                    }
                    let param = parse(
                        lex,
                        current_token,
                        current_str,
                        Token::KwLParen,
                        "(".to_string(),
                        pstate,
                    );
                    if pstate.debug {
                        println!("\x1b[32m[Fn Call] Got Result: {:?}\x1b[0m", param);
                    }
                    if param != ASTNode::None {
                        params.push(param);
                    }
                }
                //finally, put it all together (and clean up a var that might've been left set)
                if pstate.debug {
                    println!("\x1b[35m[Fn Call] Parameter List: {:?}\x1b[0m", params);
                }
                pstate.encounteredRParen = false;
                return ASTNode::FunctionCall(Box::new(ASTNode::Text(sstr.clone())), params);
            } else {
                //it's a variable
                let nx_tokp = lex.peek().unwrap();
                let nx_tok = nx_tokp.token.to_owned();
                let nx_str = nx_tokp.text.to_owned();
                match nx_tok {
                    Token::KwTerminator | Token::KwTo | Token::KwComma | Token::KwLBrace => {
                        return ASTNode::Variable(sstr.clone());
                    }
                    Token::KwLBrace => {
                        pstate.encounteredRBrace = true;
                        return ASTNode::Variable(sstr.clone());
                    }
                    Token::KwRParen => {
                        pstate.encounteredRParen = true;
                        return ASTNode::Variable(sstr.clone());
                    }
                    _ => {
                        lex.next();
                        return parse(lex, nx_tok, nx_str, stok, sstr, pstate);
                    }
                }
            }
        }

        /* Operators (All of them) */
        Token::OpAdd => {
            //pretty simple. We get the next part of the operator recursively
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            //based on the prior token type, lets figure out what the first parameter was.
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                Token::Text => ASTNode::Text(pstr.clone()),
                Token::KwTrue => ASTNode::Boolean(true),
                Token::KwFalse => ASTNode::Boolean(false),
                _ => ASTNode::None,
            };
            return ASTNode::Add(Box::new(first_param), Box::new(sec_param));
        }
        //this one's also a bit special, so it's up top
        Token::OpAssign => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            //the only valid thing to assign to is a var
            let first_param = ASTNode::Variable(pstr.clone());
            return ASTNode::Change(Box::new(first_param), Box::new(sec_param));
        }
        //KwLet technically isn't an operator, but it's pretty close to OpAssign, so it gets a home here
        Token::KwLet => {
            //next token should be the identifier
            let v_id_tokp = lex.next().unwrap();
            let v_id_str = v_id_tokp.text.to_owned();
            //skip the assignment operator (sorry branch above us!)
            lex.next();
            //after that, the expression. Recursive time!
            let nx_tokp = lex.next().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let assign = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            return ASTNode::Set(Box::new(ASTNode::Variable(v_id_str)), Box::new(assign));
        }
        //the rest of these follow the same template, more or less.
        //TODO: DRY - break getting the parameters off into it's own smaller function
        Token::OpSub => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                _ => ASTNode::None,
            };
            return ASTNode::Sub(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpMul => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                _ => ASTNode::None,
            };
            return ASTNode::Mul(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpDiv => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                _ => ASTNode::None,
            };
            return ASTNode::Div(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpModulo => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                _ => ASTNode::None,
            };
            return ASTNode::Modulo(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpAddEq => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                _ => ASTNode::None,
            };
            return ASTNode::AddEq(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpSubEq => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                _ => ASTNode::None,
            };
            return ASTNode::SubEq(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpGtCheck => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                _ => ASTNode::None,
            };
            return ASTNode::GtCheck(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpLtCheck => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                _ => ASTNode::None,
            };
            return ASTNode::LtCheck(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpGteCheck => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                _ => ASTNode::None,
            };
            return ASTNode::GteCheck(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpLteCheck => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                _ => ASTNode::None,
            };
            return ASTNode::LteCheck(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpEqCheck => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                Token::Text => ASTNode::Text(pstr.clone()),
                Token::KwTrue => ASTNode::Boolean(true),
                Token::KwFalse => ASTNode::Boolean(false),
                _ => ASTNode::None,
            };
            return ASTNode::EqCheck(Box::new(first_param), Box::new(sec_param));
        }
        Token::OpNeqCheck => {
            let nx_tokp = lex.peek().unwrap();
            let nx_tok = nx_tokp.token.to_owned();
            let nx_str = nx_tokp.text.to_owned();
            let sec_param = parse(lex, nx_tok, nx_str, stok, sstr, pstate);
            let first_param = match ptok {
                Token::Identifier => ASTNode::Variable(pstr.clone()),
                Token::Number => ASTNode::Number(pstr.parse::<f32>().unwrap()),
                Token::Text => ASTNode::Text(pstr.clone()),
                Token::KwTrue => ASTNode::Boolean(true),
                Token::KwFalse => ASTNode::Boolean(false),
                _ => ASTNode::None,
            };
            return ASTNode::NeqCheck(Box::new(first_param), Box::new(sec_param));
        }

        /* Control Flow & Functions */
        //function declaration
        Token::KwFn => {
            //first, get the id of the function
            let f_id_tokp = lex.next().unwrap();
            let f_id_str = f_id_tokp.text.to_owned();
            //we put in the id right away in case the function is recursive
            pstate.registeredFnNames.push(f_id_str.clone());
            //next, skip the LParen, and grab parameters (below snippeted from Token::Identifier)
            lex.next();
            let mut params: Vec<ASTNode> = vec![];
            let mut param_names: Vec<String> = vec![];
            loop {
                let current_tokp = lex.next().unwrap();
                let current_token = current_tokp.token.to_owned();
                let current_str = current_tokp.text.to_owned();
                //if we encounter a RParen, or our parser state claims we have,we're done.
                if current_token == Token::KwRParen {
                    break;
                } else if pstate.encounteredRParen {
                    pstate.encounteredRParen = false;
                    break;
                }
                //recursively parse this argument
                if pstate.debug {
                    println!(
                        "\x1b[36m[Fn Decl] Parsing Argument: {:?} {}\x1b[0m",
                        current_token, current_str
                    );
                }
                let param = parse(
                    lex,
                    current_token,
                    current_str.clone(),
                    Token::KwLParen,
                    "(".to_string(),
                    pstate,
                );
                if pstate.debug {
                    println!("\x1b[32m[Fn Decl] Got Result: {:?}\x1b[0m", param);
                }
                //we don't do that here.
                if param != ASTNode::None {
                    param_names.push(current_str.clone());
                    params.push(param);
                }
            }
            //reset
            pstate.encounteredRParen = false;
            //now, we get the body of the function
            //skip the LBrace, we don't care about it
            if pstate.debug {
                println!(
                    "\x1b[35m[Fn Decl] Finished getting parameters: {:?} Names: {:?}\x1b[0m",
                    params, param_names
                );
            }
            lex.next();
            //things we need for parsing the functions
            //technically registeredVarNames isn't used in favor of registereFnNames, but yknow, future proofing.
            let mut function_ast: Vec<ASTNode> = vec![];
            let mut function_pstate = ParserState {
                registeredVarNames: param_names,
                registeredFnNames: pstate.registeredFnNames.clone(),
                encounteredRBrace: false,
                encounteredRParen: false,
                debug: pstate.debug,
            };
            loop {
                let current_tokp = lex.next().unwrap();
                let current_token = current_tokp.token.to_owned();
                let current_str = current_tokp.text.to_owned();
                if pstate.debug {
                    println!(
                        "\x1b[34m[Fn Decl] Parsing Potential Body: {:?} {}\x1b[0m",
                        current_token, current_str
                    );
                }
                //if we encounter a RParen, or our parser state claims we have,we're done.
                if current_token == Token::KwRBrace {
                    break;
                } else if pstate.encounteredRBrace {
                    pstate.encounteredRParen = false;
                    break;
                }
                let isNone = current_token == Token::KwNone;
                //recursively parse this argument
                let tree = parse(
                    lex,
                    current_token,
                    current_str.clone(),
                    Token::KwLBrace,
                    "{".to_string(),
                    &mut function_pstate,
                );
                if pstate.debug {
                    println!("\x1b[36m[Fn Decl] Got Result: {:?}\x1b[0m", tree);
                }
                //make sure the only Nones we throw in are the ones we're explicitly supposed to!
                if tree == ASTNode::None {
                    if isNone {
                        function_ast.push(tree);
                    }
                } else {
                    function_ast.push(tree);
                }
            }
            if pstate.debug {
                println!("\x1b[35m[Fn Decl] Finished Reading Body\x1b[0m");
            }
            //put it all together
            ASTNode::FunctionDecl(
                Box::new(ASTNode::Text(f_id_str.clone())),
                params,
                function_ast,
            )
        }

        //if statement
        Token::KwIf => {
            //first get the condition from the next token(s)
            //a condition must be a single expressoin, we we literally just let recursiveness do our work for us.
            let nptok = lex.next().unwrap();
            let ntok = nptok.token.to_owned();
            let nstr = nptok.text.to_owned();
            let condition = parse(lex, ntok, nstr, Token::KwIf, "if".to_string(), pstate);
            if pstate.debug{
            println!(
                "\x1b[32m[IfStatement - Conditional] Got Conditional: {:?} \x1b[0m",
                condition
            );
        }
            //jump the LBrace (just like KwFn)
            //lex.next();
            //from here -> RBrace is the main ifbody
            //so, we can steal more code from KwFn!
            let mut if_ast: Vec<ASTNode> = vec![];
            loop {
                let current_tokp = lex.next().unwrap();
                let current_token = current_tokp.token.to_owned();
                let current_str = current_tokp.text.to_owned();
                if pstate.debug {
                    println!(
                        "\x1b[34m[IfStatement - If Block] Parsing Potential Body: {:?} {}\x1b[0m",
                        current_token, current_str
                    );
                }
                //if we encounter a RParen, or our parser state claims we have,we're done.
                if current_token == Token::KwRBrace {
                    if pstate.debug{
                        println!("\x1b[31mKwRBrace -> break;\x1b[0m");
                    }
                    break;
                } else if pstate.encounteredRBrace {
                    if pstate.debug{
                        println!("\x1b[31mKwRBrace Flag -> break;\x1b[0m");
                    }
                    pstate.encounteredRParen = false;
                    break;
                }
                let isNone = current_token == Token::KwNone;
                //recursively parse this argument
                let tree = parse(
                    lex,
                    current_token,
                    current_str.clone(),
                    Token::KwLBrace,
                    "{".to_string(),
                    pstate,
                );
                if pstate.debug {
                    println!(
                        "\x1b[36m[IfStatement - If Block] Got Result: {:?}\x1b[0m",
                        tree
                    );
                }
                //make sure the only Nones we throw in are the ones we're explicitly supposed to!
                if tree == ASTNode::None {
                    if isNone {
                        if_ast.push(tree);
                    }
                } else {
                    if_ast.push(tree);
                }
            }
            //cleanup
            pstate.encounteredRParen = false;
            pstate.encounteredRBrace = false;
            let nptok = lex.peek().unwrap();
            let ntok = nptok.token.to_owned();
            //if the next token (peeked to avoid screwing stuff up) is an "else", we have that to deal with.
            //otherwise, we're done here!
            let has_else = ntok == Token::KwElse;
            let mut else_ast: Vec<ASTNode> = vec![];
            if has_else {
                //forward onto the else token properly (turn the peek into the actual current state)
                lex.next();
                //now, we check the next token. If it's a KwLBrace, it's just an else clause. Otherwise, we run a recursive parse
                let etype_tokp = lex.next().unwrap();
                let etype_token = etype_tokp.token.to_owned();
                let etype_str = etype_tokp.text.to_owned();
                if etype_token == Token::KwLBrace {
                    //unadorned else clause - repeat of above w/ reading body.
                    loop {
                        let current_tokp = lex.next().unwrap();
                        let current_token = current_tokp.token.to_owned();
                        let current_str = current_tokp.text.to_owned();
                        if pstate.debug {
                            println!(
                                "\x1b[034m[IfStatement - Else Block] Parsing Potential Body: {:?} {}\x1b[0m",
                                current_token, current_str
                            );
                        }
                        //if we encounter a RParen, or our parser state claims we have,we're done.
                        if current_token == Token::KwRBrace {
                            if pstate.debug{
                        println!("\x1b[31mKwRBrace -> break;\x1b[0m");
                    }
                            break;
                        } else if pstate.encounteredRBrace {
                            if pstate.debug {
                            println!("\x1b[31mKwRBrace flag -> break;\x1b[0m");
                        }
                            pstate.encounteredRParen = false;
                            break;
                        }
                        let isNone = current_token == Token::KwNone;
                        //recursively parse this argument
                        let tree = parse(
                            lex,
                            current_token,
                            current_str.clone(),
                            Token::KwLBrace,
                            "{".to_string(),
                            pstate,
                        );
                        if pstate.debug {
                            println!(
                                "\x1b[36m[IfStatement - Else Block] Got Result: {:?}\x1b[0m",
                                tree
                            );
                        }
                        //make sure the only Nones we throw in are the ones we're explicitly supposed to!
                        if tree == ASTNode::None {
                            if isNone {
                                else_ast.push(tree);
                            }
                        } else {
                            else_ast.push(tree);
                        }
                    }
                    //cleanup
                    pstate.encounteredRParen = false;
                    pstate.encounteredRBrace = false;
                } else {
                    //recursive
                    else_ast.push(parse(
                        lex,
                        etype_token,
                        etype_str,
                        Token::KwElse,
                        "else".to_string(),
                        pstate,
                    ));
                }
            }

            ASTNode::IfStatement(Box::new(condition), if_ast, has_else, else_ast)
        }

        //loops!
        Token::KwLoop => {
            //what type of loop are you?
            let nptok = lex.next().unwrap();
            let ntok = nptok.token.to_owned();
            if ntok == Token::KwWhile {
                // a conditional loop

                //lets get ourselves the condition
                let nptok = lex.next().unwrap();
                let ntok = nptok.token.to_owned();
                let nstr = nptok.text.to_owned();
                let condition = parse(lex, ntok, nstr, Token::KwIf, "if".to_string(), pstate);
                if pstate.debug{
                println!(
                    "\x1b[32m[Loop (Conditional Variety)] Got Conditional: {:?} \x1b[0m",
                    condition
                );
            }

                //then, get the body of the loop. this is routine by now (KwIf and KwFn)
                let mut loop_ast: Vec<ASTNode> = vec![];
                loop {
                    let current_tokp = lex.next().unwrap();
                    let current_token = current_tokp.token.to_owned();
                    let current_str = current_tokp.text.to_owned();
                    if pstate.debug {
                        println!(
                        "\x1b[34m[Loop (Conditional Variety)] Parsing Potential Body: {:?} {}\x1b[0m",
                        current_token, current_str
                    );
                    }
                    //if we encounter a RParen, or our parser state claims we have,we're done.
                    if current_token == Token::KwRBrace {
                        if pstate.debug{
                        println!("\x1b[31mKwRBrace -> break;\x1b[0m");
                    }
                        break;
                    } else if pstate.encounteredRBrace {
                        if pstate.debug {
                            println!("\x1b[31mKwRBrace flag -> break;\x1b[0m");
                        }
                        pstate.encounteredRParen = false;
                        break;
                    }
                    let isNone = current_token == Token::KwNone;
                    //recursively parse this argument
                    let tree = parse(
                        lex,
                        current_token,
                        current_str.clone(),
                        Token::KwLBrace,
                        "{".to_string(),
                        pstate,
                    );
                    if pstate.debug {
                        println!(
                            "\x1b[36m[Loop (Conditional Variety)] Got Result: {:?}\x1b[0m",
                            tree
                        );
                    }
                    //make sure the only Nones we throw in are the ones we're explicitly supposed to!
                    if tree == ASTNode::None {
                        if isNone {
                            loop_ast.push(tree);
                        }
                    } else {
                        loop_ast.push(tree);
                    }
                }
                //cleanup
                pstate.encounteredRParen = false;
                pstate.encounteredRBrace = false;
                //and we're done maybe probably hopefully
                return ASTNode::ConditionalLoop(Box::new(condition), loop_ast);
            } else if ntok == Token::Identifier {
                // a incrementing loop

                //get the name of the iterator var, and the upper/lower bounds.
                let iter_id = nptok.text.to_owned();
                //skip KwIn
                lex.next();
                //get lowerbound
                let nptok = lex.next().unwrap();
                let ntok = nptok.token.to_owned();
                let nstr = nptok.text.to_owned();
                let lower_bound = parse(lex, ntok, nstr, Token::KwIn, "in".to_string(), pstate);
                //skip KwTo
                lex.next();
                //get upper bound
                let nptok = lex.next().unwrap();
                let ntok = nptok.token.to_owned();
                let nstr = nptok.text.to_owned();
                let upper_bound = parse(lex, ntok, nstr, Token::KwTo, "to".to_string(), pstate);
                //great! Now lets grab the body.
                //skip the braces - we don't need em
                lex.next();
                let mut loop_ast: Vec<ASTNode> = vec![];
                loop {
                    let current_tokp = lex.next().unwrap();
                    let current_token = current_tokp.token.to_owned();
                    let current_str = current_tokp.text.to_owned();
                    if pstate.debug {
                        println!(
                        "\x1b[34m[Loop (Incrementing Variety)] Parsing Potential Body: {:?} {}\x1b[0m",
                        current_token, current_str
                    );
                    }
                    //if we encounter a RParen, or our parser state claims we have,we're done.
                    if current_token == Token::KwRBrace {
                        if pstate.debug{
                        println!("\x1b[31mKwRBrace -> break;\x1b[0m");
                    }
                        break;
                    } else if pstate.encounteredRBrace {
                         if pstate.debug{
                        println!("\x1b[31mKwRBrace flag -> break;\x1b[0m");
                    }
                        pstate.encounteredRParen = false;
                        break;
                    }
                    let isNone = current_token == Token::KwNone;
                    //recursively parse this argument
                    let tree = parse(
                        lex,
                        current_token,
                        current_str.clone(),
                        Token::KwLBrace,
                        "{".to_string(),
                        pstate,
                    );
                    if pstate.debug {
                        println!(
                            "\x1b[36m[Loop (Incrementing Variety)] Got Result: {:?}\x1b[0m",
                            tree
                        );
                    }
                    //make sure the only Nones we throw in are the ones we're explicitly supposed to!
                    if tree == ASTNode::None {
                        if isNone {
                            loop_ast.push(tree);
                        }
                    } else {
                        loop_ast.push(tree);
                    }
                }
                //cleanup
                pstate.encounteredRParen = false;
                pstate.encounteredRBrace = false;
                return ASTNode::IncrementingLoop(Box::new(ASTNode::Variable(iter_id.clone())),Box::new(lower_bound),Box::new(upper_bound),loop_ast);
            }
            ASTNode::None
        }
        _ => ASTNode::None,
    }
}

#[derive(Logos, Debug, PartialEq, Clone)]
enum Token {
    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)] //whitespace and other undesirables
    Error,
    //declaration keywords / other important thinggs
    #[token("let")]
    KwLet,
    #[token("fn")]
    KwFn,
    #[token("if")]
    KwIf,
    #[token("else")]
    KwElse,
    #[token("loop")]
    KwLoop,
    #[token("break")]
    KwBreak,
    #[token("while")]
    KwWhile,
    #[token("true")]
    KwTrue,
    #[token("false")]
    KwFalse,
    #[token("none")]
    KwNone,
    #[token("in")]
    KwIn,
    #[token("to")]
    KwTo,
    #[token(";")]
    KwTerminator,
    #[token(",")]
    KwComma,
    #[token("(")]
    KwLParen,
    #[token(")")]
    KwRParen,
    #[token("{")]
    KwLBrace,
    #[token("}")]
    KwRBrace,
    //operations
    #[token("+=")]
    OpAddEq,
    #[token("-=")]
    OpSubEq,
    #[token("+")]
    OpAdd,
    #[token("-")]
    OpSub,
    #[token("*")]
    OpMul,
    #[token("/")]
    OpDiv,
    #[token("==")]
    OpEqCheck,
    #[token("=")]
    OpAssign,
    #[token("!=")]
    OpNeqCheck,
    #[token(">")]
    OpGtCheck,
    #[token("<")]
    OpLtCheck,
    #[token(">=")]
    OpGteCheck,
    #[token("<=")]
    OpLteCheck,
    #[token("%")]
    OpModulo,
    #[regex("(\"([^\"]+)\")")]
    Text,
    //good ol text & stuffs
    #[regex("[a-zA-Z]+")]
    Identifier,
    #[regex("[0-9]+")]
    Number,
    //le big regexp
    #[regex(r"\d+\.?\d+")]
    DecimalNumber,
}

#[derive(Debug, Clone, PartialEq)]
enum ASTNode {
    //at the end of a "branch" of our tree.
    None,
    //the "atoms" of the language.
    Text(String),
    Number(f32),
    Boolean(bool),
    //the str is the id
    Variable(String),
    //assignment - id and the expression to be assigned.
    Set(Box<ASTNode>, Box<ASTNode>),
    //like set, but for vars that already exist
    Change(Box<ASTNode>, Box<ASTNode>),
    //operations
    Add(Box<ASTNode>, Box<ASTNode>),
    AddEq(Box<ASTNode>, Box<ASTNode>),
    Sub(Box<ASTNode>, Box<ASTNode>),
    SubEq(Box<ASTNode>, Box<ASTNode>),
    Mul(Box<ASTNode>, Box<ASTNode>),
    Div(Box<ASTNode>, Box<ASTNode>),
    EqCheck(Box<ASTNode>, Box<ASTNode>),
    NeqCheck(Box<ASTNode>, Box<ASTNode>),
    GtCheck(Box<ASTNode>, Box<ASTNode>),
    LtCheck(Box<ASTNode>, Box<ASTNode>),
    GteCheck(Box<ASTNode>, Box<ASTNode>),
    LteCheck(Box<ASTNode>, Box<ASTNode>),
    Modulo(Box<ASTNode>, Box<ASTNode>),
    //functions
    FunctionCall(Box<ASTNode>, Vec<ASTNode>),
    //id | paramlist | body
    FunctionDecl(Box<ASTNode>, Vec<ASTNode>, Vec<ASTNode>),
    //condition | if body | has an else clause? | else body
    IfStatement(Box<ASTNode>, Vec<ASTNode>, bool, Vec<ASTNode>),
    //= Loop things
    //iter var name, lower bound, upper bound, body
    IncrementingLoop(Box<ASTNode>, Box<ASTNode>, Box<ASTNode>, Vec<ASTNode>),
    //condition, body
    ConditionalLoop(Box<ASTNode>, Vec<ASTNode>),
    //escape!
    LoopBreak,
}

#[derive(Debug, Clone)]
struct ParserState {
    registeredVarNames: Vec<String>,
    registeredFnNames: Vec<String>,
    encounteredRParen: bool,
    encounteredRBrace: bool,
    debug: bool,
}

#[derive(PartialEq, Debug)]
struct ParsableToken {
    token: Token,
    text: String,
}

#[derive(Debug, Clone)]
struct ExecutionContext {
    nVars: HashMap<String, f32>,
    sVars: HashMap<String, String>,
    functions: HashMap<String, ASTNode>,
}

fn exec(tree: ASTNode, executionContext: &mut ExecutionContext) -> ASTNode {
    match tree {
        ASTNode::Set(id, valexp) => {
            //get the id
            let mut idstr = "".to_string();
            if let ASTNode::Variable(vid) = *id {
                idstr = vid;
            }
            let val = exec(*valexp, executionContext);
            if let ASTNode::Text(text) = val {
                executionContext.sVars.insert(idstr.clone(), text);
            } else if let ASTNode::Number(num) = val {
                executionContext.nVars.insert(idstr.clone(), num);
            }
            //nothing should be relying on a var decl for a value unless your code has serious issues.
            return ASTNode::None;
        }
        ASTNode::Change(id,valexp) => {
            //get the id
            let mut idstr = "".to_string();
            if let ASTNode::Variable(vid) = *id {
                idstr = vid;
            }
            let val = exec(*valexp, executionContext);
            if let ASTNode::Text(text) = val {
                executionContext.sVars.entry(idstr.clone()).and_modify(|o| *o = text.clone()); //todo: there must be a better way!
            } else if let ASTNode::Number(num) = val {
                executionContext.nVars.entry(idstr.clone()).and_modify(|o| *o = num);
            }
            return ASTNode::None;
        }
        ASTNode::AddEq(id,valexp) => {
            let mut idstr = "".to_string();
            if let ASTNode::Variable(vid) = *id {
                idstr = vid;
            }
            let val = exec(*valexp, executionContext);
            if let ASTNode::Text(text) = val {
                executionContext.sVars.entry(idstr.clone()).and_modify(|o| *o = format!("{}{}",o,text)); //todo: there must be a better way!
            } else if let ASTNode::Number(num) = val {
                executionContext.nVars.entry(idstr.clone()).and_modify(|o| *o += num);
            }
            return ASTNode::None;
        }
         ASTNode::SubEq(id,valexp) => {
            let mut idstr = "".to_string();
            if let ASTNode::Variable(vid) = *id {
                idstr = vid;
            }
            let val = exec(*valexp, executionContext);
            if let ASTNode::Number(num) = val {
                executionContext.nVars.entry(idstr.clone()).and_modify(|o| *o -= num);
            }
            return ASTNode::None;
        }
        ASTNode::Add(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_is_num = false;
            let mut first_num = 0f32;
            let mut first_str = "".to_string();
            if let ASTNode::Number(n) = v1 {
                first_is_num = true;
                first_num = n;
            } else if let ASTNode::Text(t) = v1 {
                first_is_num = false;
                first_str = t;
            }
            let mut second_is_num = false;
            let mut second_num = 0f32;
            let mut second_str = "".to_string();
            if let ASTNode::Number(n) = v2 {
                second_is_num = true;
                second_num = n;
            } else if let ASTNode::Text(t) = v2 {
                second_is_num = false;
                second_str = t;
            }
            //now for the big addition:tm:
            if first_is_num && second_is_num {
                return ASTNode::Number(first_num + second_num);
            } else if !first_is_num && second_is_num {
                return ASTNode::Text(format!("{}{}", first_str, second_num));
            } else if first_is_num && !second_is_num {
                return ASTNode::Text(format!("{}{}", first_num, second_str));
            } else {
                return ASTNode::Text(format!("{}{}", first_str, second_str));
            }
        }
        ASTNode::EqCheck(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_is_num = false;
            let mut first_num = 0f32;
            let mut first_str = "".to_string();
            if let ASTNode::Number(n) = v1 {
                first_is_num = true;
                first_num = n;
            } else if let ASTNode::Text(t) = v1 {
                first_is_num = false;
                first_str = t;
            }
            let mut second_is_num = false;
            let mut second_num = 0f32;
            let mut second_str = "".to_string();
            if let ASTNode::Number(n) = v2 {
                second_is_num = true;
                second_num = n;
            } else if let ASTNode::Text(t) = v2 {
                second_is_num = false;
                second_str = t;
            }
            //now for the big addition:tm:
            if first_is_num && second_is_num {
                return ASTNode::Boolean(first_num == second_num);
            } else if !first_is_num && second_is_num {
                return ASTNode::Boolean(false);
            } else if first_is_num && !second_is_num {
                return ASTNode::Boolean(false);
            } else {
                return ASTNode::Boolean(first_str == second_str);
            }
        }
        ASTNode::NeqCheck(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_is_num = false;
            let mut first_num = 0f32;
            let mut first_str = "".to_string();
            if let ASTNode::Number(n) = v1 {
                first_is_num = true;
                first_num = n;
            } else if let ASTNode::Text(t) = v1 {
                first_is_num = false;
                first_str = t;
            }
            let mut second_is_num = false;
            let mut second_num = 0f32;
            let mut second_str = "".to_string();
            if let ASTNode::Number(n) = v2 {
                second_is_num = true;
                second_num = n;
            } else if let ASTNode::Text(t) = v2 {
                second_is_num = false;
                second_str = t;
            }
            //now for the big addition:tm:
            if first_is_num && second_is_num {
                return ASTNode::Boolean(first_num != second_num);
            } else if !first_is_num && second_is_num {
                return ASTNode::Boolean(true);
            } else if first_is_num && !second_is_num {
                return ASTNode::Boolean(true);
            } else {
                return ASTNode::Boolean(first_str != second_str);
            }
        }
        ASTNode::Sub(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_num = 0f32;
            if let ASTNode::Number(n) = v1 {
                first_num = n;
            }
            let mut second_num = 0f32;
            if let ASTNode::Number(n) = v2 {
                second_num = n;
            }
            return ASTNode::Number(first_num - second_num);
        }
        ASTNode::Mul(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_num = 0f32;
            if let ASTNode::Number(n) = v1 {
                first_num = n;
            }
            let mut second_num = 0f32;
            if let ASTNode::Number(n) = v2 {
                second_num = n;
            }
            return ASTNode::Number(first_num * second_num);
        }
        ASTNode::Div(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_num = 0f32;
            if let ASTNode::Number(n) = v1 {
                first_num = n;
            }
            let mut second_num = 0f32;
            if let ASTNode::Number(n) = v2 {
                second_num = n;
            }
            return ASTNode::Number(first_num / second_num);
        }
        ASTNode::Modulo(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_num = 0f32;
            if let ASTNode::Number(n) = v1 {
                first_num = n;
            }
            let mut second_num = 0f32;
            if let ASTNode::Number(n) = v2 {
                second_num = n;
            }
            return ASTNode::Number(first_num % second_num);
        }
        ASTNode::GtCheck(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_num = 0f32;
            if let ASTNode::Number(n) = v1 {
                first_num = n;
            }
            let mut second_num = 0f32;
            if let ASTNode::Number(n) = v2 {
                second_num = n;
            }
            return ASTNode::Boolean(first_num > second_num);
        }
        ASTNode::GteCheck(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_num = 0f32;
            if let ASTNode::Number(n) = v1 {
                first_num = n;
            }
            let mut second_num = 0f32;
            if let ASTNode::Number(n) = v2 {
                second_num = n;
            }
            return ASTNode::Boolean(first_num >= second_num);
        }
        ASTNode::LtCheck(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_num = 0f32;
            if let ASTNode::Number(n) = v1 {
                first_num = n;
            }
            let mut second_num = 0f32;
            if let ASTNode::Number(n) = v2 {
                second_num = n;
            }
            return ASTNode::Boolean(first_num < second_num);
        }
        ASTNode::LteCheck(p1, p2) => {
            //actually get ourselves some values
            let v1 = exec(*p1, executionContext);
            let v2 = exec(*p2, executionContext);
            let mut first_num = 0f32;
            if let ASTNode::Number(n) = v1 {
                first_num = n;
            }
            let mut second_num = 0f32;
            if let ASTNode::Number(n) = v2 {
                second_num = n;
            }
            return ASTNode::Boolean(first_num <= second_num);
        }
        ASTNode::FunctionDecl(id, params, body) => {
            let mut idstr: String = "".to_string();
            //grab the id
            if let ASTNode::Text(idtxt) = *id {
                idstr = idtxt;
            }
            executionContext.functions.insert(
                idstr,
                ASTNode::FunctionDecl(Box::new(ASTNode::None), params.clone(), body.clone()),
            );
            //TODO: make this return a variable pointing to the function
            ASTNode::None
        }
        ASTNode::FunctionCall(id, params) => {
            // println!("Calling Function {:?} w/ params {:?}",id,params);
            //get the id
            let mut idstr = "".to_string();
            if let ASTNode::Text(vid) = *id {
                idstr = vid;
            }

            //first, check for calling a builtin
            if idstr == "print" {
                //get our value
                let val = exec(params[0].clone(), executionContext);
                if let ASTNode::Number(num) = val {
                    println!("{}", num);
                } else if let ASTNode::Text(text) = val {
                    println!("{}", text);
                } else if let ASTNode::Boolean(b) = val {
                    println!("{}", b);
                }
                //should either be a number or a string or a bool. those are the only 3 cases we'll handle.
            } else if idstr == "return" {
                return exec(params[0].clone(), executionContext);
            }
            //otherwise, perform a function table lookup
            else {
                let functionData = executionContext.functions[&idstr].clone();
                if let ASTNode::FunctionDecl(_, fparam, ftrees) = functionData {
                    let mut f_execcontext = ExecutionContext {
                        nVars: HashMap::new(),
                        sVars: HashMap::new(),
                        functions: executionContext.functions.clone(),
                    };
                    //populate the variables (annoyance moment)
                    for i in 0..fparam.len() {
                        //get the id of the variable
                        let mut vid = "".to_string();
                        if let ASTNode::Variable(idstr) = &fparam[i] {
                            vid = idstr.clone();
                        }
                        //this is our way of getting around type specification. Pull it from the node type of the parameter
                        if let ASTNode::Number(num) = params[i] {
                            f_execcontext.nVars.insert(vid, num);
                        } else if let ASTNode::Text(string) = &params[i] {
                            f_execcontext.sVars.insert(vid, string.clone());
                        } else if let ASTNode::Variable(vid) = &params[i] {
                            //you're a tricky one, you know that?
                            if executionContext.nVars.contains_key(vid) {
                                f_execcontext
                                    .nVars
                                    .insert(vid.clone(), executionContext.nVars[vid]);
                            } else if executionContext.sVars.contains_key(vid) {
                                f_execcontext
                                    .sVars
                                    .insert(vid.clone(), executionContext.sVars[vid].clone());
                            }
                        }
                    }
                    //println!("Function Exec Context: {:?}",f_execcontext);
                    //next, run the function execution - we return the result of the last function call (a rather rust-like convention honestly)
                    let mut ret_val = ASTNode::None;
                    for tree in ftrees {
                        //  println!("[Fn] ExecTree: {:?}",tree);
                        ret_val = exec(tree, &mut f_execcontext);
                    }
                    return ret_val;
                }
            }

            ASTNode::None
        }
        ASTNode::Variable(id) => {
            if executionContext.nVars.contains_key(&id) {
                return ASTNode::Number(executionContext.nVars[&id].clone());
            } else if executionContext.sVars.contains_key(&id) {
                return ASTNode::Text(executionContext.sVars[&id].clone());
            }
            ASTNode::None
        }
        ASTNode::IfStatement(condition, ifbody, haselse, elsebody) => {
            //= first, evaluate the conditino
            let ceval = exec(*condition, executionContext);
            //depending on which atomic type this is, we evaluate if it's true or not differently
            let condition_tval: bool;
            if let ASTNode::Number(num) = ceval {
                condition_tval = num != 0.0;
                //println!("Converting number to boolean: {} -> {}",num,num != 0.0);
            } else if let ASTNode::Boolean(b) = ceval {
                condition_tval = b
            } else {
                condition_tval = false; //heaven knows what you've done, but it ain't true.
            }
            //= now, execute the if block (or dont)
            if condition_tval {
                for tree in ifbody {
                    exec(tree, executionContext);
                }
            } else {
                //we're doing smth else
                if haselse {
                    for tree in elsebody {
                        exec(tree, executionContext);
                    }
                }
            }
            ASTNode::None
        }
        ASTNode::ConditionalLoop(condition, loopbody) => {
            loop {
                //check if we should be looping
                let ceval = if let ASTNode::Boolean(b) = exec(*condition.clone(), executionContext)
                {
                    b
                } else {
                    false
                };
                if ceval {
                    //run a loop iteration (ergo, execute the trees!)
                    for tree in loopbody.clone() {
                        let val = exec(tree, executionContext);
                        if val == ASTNode::LoopBreak {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            ASTNode::None
        }
        ASTNode::IncrementingLoop(a_itername, a_lowerbound, a_upperbound, loopbody) => {
            //pull out our vars from all these boxes
            //who says unboxing things in the holidays has to be limited to physical objects? (~ me, 12-27-21)
            let mut itername = String::new();
            let mut lowerbound = 0f32;
            let mut upperbound = 0f32;
            if let ASTNode::Variable(text) = *a_itername {
                itername = text;
            }
            if let ASTNode::Number(n) = *a_lowerbound {
                lowerbound = n;
            }
            if let ASTNode::Number(n) = *a_upperbound {
                upperbound = n;
            }
            //next construct the basic execution context for the loop
            let mut lpexec_context = executionContext.clone();
            lpexec_context.nVars.insert(itername.clone(), lowerbound);
            //now, iterate
            for i in lowerbound as i32..upperbound as i32 {
                *lpexec_context.nVars.get_mut(&itername).unwrap() = i as f32;
                for tree in loopbody.clone() {
                    let val = exec(tree, &mut lpexec_context);
                    if val == ASTNode::LoopBreak {
                        break;
                    }
                }
            }
            ASTNode::None
        }

        //the atomic types just get mirrored through
        ASTNode::Number(_) | ASTNode::Text(_) | ASTNode::Boolean(_) => return tree,
        _ => ASTNode::None,
    }
}

fn preprocess(file:String) -> String{
    let mut processed_file = "".to_string();
    //ew windows
    let cleaned = file.replace("\r", "");

    //the heart of this is pretty simple. we go through line by line, check if theres something we should care abot
    // if there is, we do stuff. otherwise we dont.
    let lines = cleaned.split("\n");
    for line in lines{
        let should_include_line = !line.starts_with("#");
        if should_include_line{
            //we conviently leave the /n out.
            processed_file += line;
        }
    }
    return processed_file;
}

const USE_ARGS: bool = true;

fn main() {
    println!("\x1b[97mSKCore | A (modified) sack interpreter in Rust. | (C) Luminoso 2021 (barely!) / All Rights Reserved\x1b[0m");
    /* Get Our File */
    let mut file_contents: String;
    let mut debug: bool = false;
    if USE_ARGS {
        let args: Vec<String> = std::env::args().collect();
        println!("getting file from {}", args.get(1).unwrap());
        file_contents = fs::read_to_string(args.get(1).unwrap()).unwrap();
        if args.get(2) != None {
            debug = true;
        } else {
            debug = false;
        }
    } else {
        file_contents =
            fs::read_to_string("C:/workspace/programming/rust/scriptinglang/test.sk").unwrap();
    }
    file_contents = preprocess(file_contents);
    /* Lex / Parse */
    //basically convert our code to something executable.

    //= lex
    let mut lex = Token::lexer(&file_contents);
    //why have a tree when you can have an O R C H A R D   O F   C U R S E D N E S S (~ Me, 12-26-21)
    let mut trees: Vec<ASTNode> = vec![];

    let mut pstate = ParserState {
        registeredVarNames: vec![],
        registeredFnNames: vec!["print".to_string(), "return".to_string()],
        encounteredRParen: false,
        encounteredRBrace: false,
        debug: debug,
    };
    let mut tokens: Vec<ParsableToken> = vec![];
    loop {
        let tok_o = lex.next();
        if tok_o != None {
            tokens.push(ParsableToken {
                token: tok_o.unwrap(),
                text: lex.slice().to_string(),
            });
        } else {
            break;
        }
    }
    // println!("TOKENS:\n{:?}",tokens);
    let mut tok_iter = tokens.iter().peekable();
    loop {
        let tok_o = tok_iter.next();
        if tok_o != None {
            let tok_u = tok_o.unwrap();
            let tok = tok_u.token.to_owned();
            let sstr = tok_u.text.to_owned();

            trees.push(parse(
                &mut tok_iter,
                tok,
                sstr,
                Token::Error,
                "".to_string(),
                &mut pstate,
            ));
        } else {
            break;
        }
    }
    println!("*** AST Generation Complete ***");
    // if debug {
    //     println!("{:?}", trees);
    // }
    println!("*** Executing... ***");

    //== Execute
    // we iterate down through each line of the tree, and execute.
    // we keep a hashmap of vars.

    let mut execcontext = ExecutionContext {
        nVars: HashMap::new(),
        sVars: HashMap::new(),
        functions: HashMap::new(),
    };

    for tree in trees {
        if debug {
            println!("\x1b[32m Executing tree {:?}\x1b[0m", &tree);
        }
        let _res = exec(tree, &mut execcontext);
        if debug {
            println!("\x1b[2;37m Execution Context: {:?}\x1b[0m", &execcontext)
        }
    }
}
