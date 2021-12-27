#![allow(dead_code)]
#![allow(unused_assignments)]
#![allow(non_snake_case)]
/*
A Tiny Scripting Language
---------
(C) Luminoso 2021 / All Rights Reserved
*/
use logos::{Lexer, Logos};
use std::collections::HashMap;
use std::fs;

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
    #[token(";")]
    KwTerminator,
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
    #[token("%")]
    OpModulo,
    #[regex("(\"([^\"]+)\")")]
    Text,
    //good ol text & stuffs
    #[regex("[a-zA-Z]+")]
    Identifier,
    #[token(".")]
    Dot,
    #[regex("[0-9]+")]
    Number,
    //le big regexp
    #[regex(r"\d+\.?\d+")]
    DecimalNumber,
}

#[derive(Debug, Clone)]
enum ASTNode {
    //at the end of a "branch" of our tree.
    None,
    //the "atoms" of the language.
    Text(String),
    Number(f32),
    //the str is the id
    Variable(String),
    //assignment - id and the expression to be assigned.
    Set(Box<ASTNode>, Box<ASTNode>),
    //operations
    Add(Box<ASTNode>, Box<ASTNode>),
    Sub(Box<ASTNode>, Box<ASTNode>),
    Mul(Box<ASTNode>, Box<ASTNode>),
    Div(Box<ASTNode>, Box<ASTNode>),
    //functions
    FunctionCall(Box<ASTNode>, Vec<ASTNode>),
    //id | paramlist | body
    FunctionDecl(Box<ASTNode>,Vec<ASTNode>,Vec<ASTNode>)
}

#[derive(Debug)]
struct ParserState {
    registeredVarNames: Vec<String>,
    registeredFnNames: Vec<String>,
    encounteredRParen: bool,
}

fn parse(
    lex: &mut logos::Lexer<'_, Token>,
    stok: Token,
    sstr: String,
    pstate: &mut ParserState,
) -> ASTNode {
    match stok {
        Token::KwLet => {
            lex.next();
            let id = lex.slice().to_string();
            //from now on, any string that says this will be taken to be a variable
            pstate.registeredVarNames.push(id.clone());
            //skip over the assignment token, we don't actually need it...
            lex.next();
            //recursive time!
            let ntok = lex.next().unwrap();
            let nstr = lex.slice().to_string();
            let assignment = parse(lex, ntok, nstr, pstate);
            return ASTNode::Set(Box::new(ASTNode::Variable(id)), Box::new(assignment));
        }
        Token::Number => {
            let ntok = lex.next().unwrap();
            let nstr = lex.slice().to_string();
            if ntok == Token::KwTerminator || ntok == Token::Error {
                return ASTNode::Number(sstr.parse::<f32>().unwrap());
            } else {
                let atok = lex.next().unwrap();
                let astr = lex.slice().to_string();
                let assign = parse(lex, atok, astr, pstate);
                match ntok {
                    Token::OpAdd => {
                        return ASTNode::Add(
                            Box::new(ASTNode::Number(sstr.parse::<f32>().unwrap())),
                            Box::new(assign),
                        )
                    }
                    Token::OpSub => {
                        return ASTNode::Sub(
                            Box::new(ASTNode::Number(sstr.parse::<f32>().unwrap())),
                            Box::new(assign),
                        )
                    }
                    Token::OpMul => {
                        return ASTNode::Mul(
                            Box::new(ASTNode::Number(sstr.parse::<f32>().unwrap())),
                            Box::new(assign),
                        )
                    }
                    Token::OpDiv => {
                        return ASTNode::Div(
                            Box::new(ASTNode::Number(sstr.parse::<f32>().unwrap())),
                            Box::new(assign),
                        )
                    }
                    _ => ASTNode::Number(sstr.parse::<f32>().unwrap()),
                }
            }
        }
        Token::DecimalNumber => {
            let ntok = lex.next().unwrap();
            if ntok == Token::KwTerminator || ntok == Token::Error {
                return ASTNode::Number(sstr.parse::<f32>().unwrap());
            } else {
                let atok = lex.next().unwrap();
                let astr = lex.slice().to_string();
                let assign = parse(lex, atok, astr, pstate);
                match ntok {
                    Token::OpAdd => {
                        return ASTNode::Add(
                            Box::new(ASTNode::Number(sstr.parse::<f32>().unwrap())),
                            Box::new(assign),
                        )
                    }
                    Token::OpSub => {
                        return ASTNode::Sub(
                            Box::new(ASTNode::Number(sstr.parse::<f32>().unwrap())),
                            Box::new(assign),
                        )
                    }
                    Token::OpMul => {
                        return ASTNode::Mul(
                            Box::new(ASTNode::Number(sstr.parse::<f32>().unwrap())),
                            Box::new(assign),
                        )
                    }
                    Token::OpDiv => {
                        return ASTNode::Div(
                            Box::new(ASTNode::Number(sstr.parse::<f32>().unwrap())),
                            Box::new(assign),
                        )
                    }
                    _ => ASTNode::Number(sstr.parse::<f32>().unwrap()),
                }
            }
        }
        //return ASTNode::Number(sstr.parse::<f32>().unwrap()),
        Token::Text => {
            //TODO: trim prefix/suffix w/ a slice instead of replace.
            let ntok = lex.next().unwrap();
            let _nstr = lex.slice().to_string();
            if ntok == Token::KwTerminator || ntok == Token::Error {
                return ASTNode::Text(sstr.replace("\"", ""));
            } else {
                let atok = lex.next().unwrap();
                let astr = lex.slice().to_string();
                let assign = parse(lex, atok, astr, pstate);

                match ntok {
                    Token::OpAdd => {
                        return ASTNode::Add(Box::new(ASTNode::Text(sstr.replace("\"", ""))), Box::new(assign))
                    }
                    Token::OpSub => {
                        return ASTNode::Sub(Box::new(ASTNode::Text(sstr.replace("\"", ""))), Box::new(assign))
                    }
                    Token::OpMul => {
                        return ASTNode::Mul(Box::new(ASTNode::Text(sstr.replace("\"", ""))), Box::new(assign))
                    }
                    Token::OpDiv => {
                        return ASTNode::Div(Box::new(ASTNode::Text(sstr.replace("\"", ""))), Box::new(assign))
                    }
                    _ => ASTNode::Text(sstr.replace("\"", "")),
                }
            }
        }
        Token::KwFn => {
            // to start, lets grab the id
            lex.next();
            let func_id = lex.slice().to_string();
            pstate.registeredFnNames.push(func_id.clone());
            //the next token should be an lparen, we skip it
            //TODO: verify syntax!
            lex.next();
            //now we loop to get the local variables of the function.
            let mut local_vars: Vec<ASTNode> = vec![];
            let mut local_var_ids:Vec<String> = vec![];
            loop {
                let nxtok = lex.next().unwrap();
                if nxtok == Token::KwRParen {
                    //end of param list
                    break;
                } else if nxtok == Token::Identifier {
                    let nxstr = lex.slice().to_string();
                    //it's a parameter name!
                    local_var_ids.push(nxstr.clone());
                    local_vars.push(ASTNode::Variable(nxstr.clone()));
                }
            }
            //skip the next token, it's the LBrace. the rest of this is actual body code, we'll read this till the RBrace.
            lex.next();
            let mut function_ast:Vec<ASTNode> = vec![];
            let mut function_pstate = ParserState{
                registeredVarNames: local_var_ids,
                registeredFnNames: pstate.registeredFnNames.clone(),
                encounteredRParen: false,
            };
            loop {
                let tok_o = lex.next();
                if tok_o != None {
                    let tok = tok_o.unwrap();
                    let sstr = lex.slice().to_string();
                    if tok == Token::KwRBrace {
                        break;
                    }

                    function_ast.push(parse(lex, tok, sstr, &mut function_pstate));
                } else {
                    break;
                }
            }
            //println!("fn {} has params {:?} and body {:?}",func_id,local_vars,function_ast);
            return ASTNode::FunctionDecl(Box::new(ASTNode::Text(func_id)),local_vars,function_ast);
        }
        Token::Identifier => {
            let ntok = lex.next().unwrap();
            let _nstr = lex.slice().to_string();
            if ntok == Token::KwTerminator || ntok == Token::Error || ntok == Token::KwRParen {
                pstate.encounteredRParen = true;
                return ASTNode::Variable(sstr);
            } else {
                //**are thou a function, or are thou not a function. That is the question eternal.**
                if pstate.registeredFnNames.iter().any(|name| name == &sstr) {
                    //it's a function name. It shall be treated as one!
                    //if the next token is a LParen, we have ourselves a call
                    if ntok == Token::KwLParen {
                        let mut params: Vec<ASTNode> = vec![];
                        loop {
                            let nxtok_o = lex.next();
                            if nxtok_o == None {
                                break;
                            }
                            if pstate.encounteredRParen {
                                pstate.encounteredRParen = false;
                                break;
                            }
                            let nxtok = nxtok_o.unwrap();
                            let nxstr = lex.slice().to_string();
                            if nxtok == Token::KwRParen {
                                break;
                            }
                            //println!("{:?} {}", nxtok, nxstr);
                            //gib token pls
                            params.push(parse(lex, nxtok, nxstr, pstate));
                        }
                        println!("Pulled func invoke of {} Params: {:?}",sstr,params);
                        return ASTNode::FunctionCall(Box::new(ASTNode::Text(sstr)), params);
                    } else {
                        //what on earth are you doing?
                        return ASTNode::None;
                    }
                } else {
                    let atok = lex.next().unwrap();
                    let astr = lex.slice().to_string();

                    let assign = parse(lex, atok, astr, pstate);

                    match ntok {
                        Token::OpAdd => {
                            return ASTNode::Add(
                                Box::new(ASTNode::Variable(sstr)),
                                Box::new(assign),
                            )
                        }
                        Token::OpSub => {
                            return ASTNode::Sub(
                                Box::new(ASTNode::Variable(sstr)),
                                Box::new(assign),
                            )
                        }
                        Token::OpMul => {
                            return ASTNode::Mul(
                                Box::new(ASTNode::Variable(sstr)),
                                Box::new(assign),
                            )
                        }
                        Token::OpDiv => {
                            return ASTNode::Div(
                                Box::new(ASTNode::Variable(sstr)),
                                Box::new(assign),
                            )
                        }
                        _ => ASTNode::Variable(sstr),
                    }
                }
            }
        }

        _ => return ASTNode::None,
    }
}

#[derive(Debug)]
struct ExecutionContext {
    nVars: HashMap<String, f32>,
    sVars: HashMap<String, String>,
    functions: HashMap<String,ASTNode>
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
        ASTNode::FunctionDecl(id,params,body) => {
            let mut idstr:String = "".to_string();
            //grab the id
            if let ASTNode::Text(idtxt) = *id{
                idstr = idtxt;
            }
            executionContext.functions.insert(idstr,ASTNode::FunctionDecl(Box::new(ASTNode::None),params.clone(),body.clone()));
            //TODO: make this return a variable pointing to the function
            ASTNode::None
        }
        ASTNode::FunctionCall(id, params) => {
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
                }
                //should either be a number or a string. those are the only 2 cases we'll handle.
            } else if idstr == "return"{
                 return exec(params[0].clone(), executionContext);
            }
            //otherwise, perform a function table lookup
            else {
                let functionData = executionContext.functions[&idstr].clone();
                if let ASTNode::FunctionDecl(_,fparam,ftrees) = functionData{
                    let mut f_execcontext = ExecutionContext{
                        nVars: HashMap::new(),
                        sVars: HashMap::new(),
                        functions: executionContext.functions.clone(),
                    };
                    //populate the variables (annoyance moment)
                    for i in 0..fparam.len(){
                        //get the id of the variable
                        let mut vid = "".to_string();
                        if let ASTNode::Variable(idstr) = &fparam[i]{
                            vid = idstr.clone();

                        }
                        //this is our way of getting around type specification. Pull it from the node type of the parameter
                       if let ASTNode::Number(num) = params[i]{
                            f_execcontext.nVars.insert(vid, num);

                       } else  if let ASTNode::Text(string) = &params[i] {
                            f_execcontext.sVars.insert(vid, string.clone());
                       }
                    }
                    println!("function execution context: {:?}",f_execcontext);
                    //next, run the function execution - we return the result of the last function call (a rather rust-like convention honestly)
                    let mut ret_val = ASTNode::None;
                    for tree in ftrees{
                        ret_val = exec(tree,&mut f_execcontext);
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
        //the atomic types just get mirrored through
        ASTNode::Number(_) | ASTNode::Text(_) => return tree,
        _ => ASTNode::None,
    }
}

fn main() {
    /* Get Our File */
    //TODO: replace w/ pulling from args
    let file_contents =
        fs::read_to_string("C:/workspace/programming/rust/scriptinglang/test.sk").unwrap();
    /* Lex / Parse */
    //basically convert our code to something executable.

    //= lex
    let mut lex = Token::lexer(&file_contents);
    //why have a tree when you can have an O R C H A R D   O F   C U R S E D N E S S (~ Me, 12-26-21)
    let mut trees: Vec<ASTNode> = vec![];

    let mut pstate = ParserState {
        registeredVarNames: vec![],
        registeredFnNames: vec!["print".to_string()],
        encounteredRParen: false,
    };
    loop {
        println!("Parsing Context: {:?}",pstate);
        let tok_o = lex.next();
        if tok_o != None {
            let tok = tok_o.unwrap();
            let sstr = lex.slice().to_string();

            trees.push(parse(&mut lex, tok, sstr, &mut pstate));
        } else {
            break;
        }
    }
    println!("*** AST Generation Complete ***");
    println!("*** Executing... ***");

    //== Execute
    // we iterate down through each line of the tree, and execute.
    // we keep a hashmap of vars.
    // TODO: Think about what function execution semantics are going to look like. Do we pull them aside at parsetime? Insert them into the AST?
    // the latter seems more elegant

    let mut execcontext = ExecutionContext {
        nVars: HashMap::new(),
        sVars: HashMap::new(),
        functions: HashMap::new(),
    };

    for tree in trees {
        println!("Executing tree {:?}", &tree);
        let res = exec(tree, &mut execcontext);
        println!("Execution Context: {:?}", &execcontext)
    }
}
