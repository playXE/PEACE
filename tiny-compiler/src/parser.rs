use std::{char, error::Error, fmt, iter::Peekable, str::Chars};

#[derive(Debug, Clone)]
pub enum LexError {
    UnexpectedChar,
    MalformedEscapeSequence,
    MalformedNumber,
    MalformedChar,
    Nothing,
}

impl Error for LexError {
    fn description(&self) -> &str {
        match *self {
            LexError::UnexpectedChar => "Unexpected character in input",
            LexError::MalformedEscapeSequence => "Unexpected values in escape sequence",
            LexError::MalformedNumber => "Unexpected characters in number",
            LexError::MalformedChar => "Char constant not a single character",
            LexError::Nothing => "This error is for internal use only",
        }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[derive(Debug)]
pub enum ParseError {
    BadInput,
    InputPastEndOfFile,
    UnknownOperator,
    MissingRParen,
    MissingLParen,
    MissingLCurly,
    MissingSemicolon,
    MissingRCurly,
    MissingRSquare,
    MalformedCallExpr,
    MalformedIndexExpr,
    VarExpectsIdentifier,
    FnMissingName,
    ClassMissingName,
    FnMissingParams,
}

impl Error for ParseError {
    fn description(&self) -> &str {
        match *self {
            ParseError::BadInput => "Unparseable characters in the input stream",
            ParseError::InputPastEndOfFile => "Input past end of file",
            ParseError::UnknownOperator => "Unknown operator",
            ParseError::MissingRParen => "Expected ')'",
            ParseError::MissingSemicolon => "Expected ';'",
            ParseError::MissingLParen => "Expected  '('",
            ParseError::MissingLCurly => "Expected '{'",
            ParseError::MissingRCurly => "Expected '}'",
            ParseError::MissingRSquare => "Expected ']'",
            ParseError::MalformedCallExpr => "Call contains bad expression",
            ParseError::MalformedIndexExpr => "Indexing expression missing correct index",
            ParseError::VarExpectsIdentifier => "'var' expects the name of a variable",
            ParseError::FnMissingName => "Function declaration is missing name",
            ParseError::FnMissingParams => "Function declaration is missing parameters",
            ParseError::ClassMissingName => "Class missing name",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: Box<Expr>,
    pub params: Vec<String>,
    pub body: Box<Stmt>,
    pub params_: HashMap<String, TypeSpec>,
    pub ret_ty: TypeSpec,
}

#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: Box<Expr>,
    pub vars: Vec<(String, Option<Expr>)>,
    pub methods: Vec<FnDef>,
}

#[derive(Clone, Debug)]
pub struct TypeSpec {
    pub name: String,
    pub is_ref: bool,
}

#[derive(Clone, Debug)]
pub enum Global {
    ClassDefinition(ClassDef),
    FnDefenition(FnDef),
    Variable(Stmt),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    If(Box<Expr>, Box<Stmt>),
    IfElse(Box<Expr>, Box<Stmt>, Box<Stmt>),
    While(Box<Expr>, Box<Stmt>),
    For(Box<Stmt>, Box<Expr>, Box<Expr>, Box<Stmt>),
    Loop(Box<Stmt>),
    Var(String, TypeSpec, Option<Box<Expr>>),
    Block(Vec<Stmt>),
    Expr(Box<Expr>),
    Label(String),
    Goto(String),
    Break,
    Return,
    ReturnWithVal(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntConst(i64),
    FloatConst(f64),
    Identifier(String),
    RefIdentifier(String),
    CharConst(char),
    StringConst(String),
    FnCall(String, Vec<Expr>),
    Assignment(Box<Expr>, Box<Expr>),
    Dot(Box<Expr>, Box<Expr>),
    Index(String, Box<Expr>),
    Array(Vec<Expr>),
    New(String, Vec<Expr>),
    Cast(TypeSpec, Box<Expr>),
    True,
    This,
    False,
    Unit,
    Op(Op, Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn is_call(&self) -> bool {
        match self {
            Expr::FnCall(_, _) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Token {
    IntConst(i64),
    FloatConst(f64),
    Identifier(String),
    CharConst(char),
    StringConst(String),
    Ref,
    Cast,
    Null,
    Label,
    For,
    NewLine,
    End,
    Enum,
    This,
    Goto,
    LCurly,
    RCurly,
    LParen,
    RParen,
    New,
    LSquare,
    RSquare,
    Plus,
    UnaryPlus,
    Minus,
    UnaryMinus,
    Multiply,
    Divide,
    Semicolon,
    Colon,
    Comma,
    Period,
    Equals,
    True,
    False,
    Var,
    Local,
    If,
    Else,
    While,
    Loop,
    LessThan,
    GreaterThan,
    Bang,
    LessThanEqual,
    GreaterThanEqual,
    EqualTo,
    NotEqualTo,
    Pipe,
    Or,
    Ampersand,
    And,
    Fn,
    Class,
    Break,
    Return,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    LeftShiftAssign,
    RightShiftAssign,
    AndAssign,
    OrAssign,
    XOrAssign,
    LeftShift,
    RightShift,
    XOr,
    Modulo,
    ModuloAssign,
    PowerOf,
    PowerOfAssign,
    LexErr(LexError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Div,
    Mul,
    Eq,
    Ge,
    Le,
    Lt,
    Gt,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    And,
    Or,
    Shl,
    Shr,
    Access,
    Isa,
    Ne,
}

impl Token {
    // if another operator is after these, it's probably an unary operator
    // not sure about fn's name
    pub fn is_next_unary(&self) -> bool {
        use self::Token::*;

        match *self {
            LCurly           | // (+expr) - is unary
            // RCurly           | {expr} - expr not unary & is closing
            LParen           | // {-expr} - is unary
            // RParen           | (expr) - expr not unary & is closing
            LSquare          | // [-expr] - is unary
            // RSquare          | [expr] - expr not unary & is closing
            Plus             |
            UnaryPlus        |
            Minus            |
            UnaryMinus       |
            Multiply         |
            Divide           |
            Colon            |
            Comma            |
            Period           |
            Equals           |
            LessThan         |
            GreaterThan      |
            Bang             |
            LessThanEqual    |
            GreaterThanEqual |
            EqualTo          |
            NotEqualTo       |
            Pipe             |
            Or               |
            Ampersand        |
            And              |
            If               |
            While            |
            PlusAssign       |
            MinusAssign      |
            MultiplyAssign   |
            DivideAssign     |
            LeftShiftAssign  |
            RightShiftAssign |
            AndAssign        |
            OrAssign         |
            XOrAssign        |
            LeftShift        |
            RightShift       |
            XOr              |
            Modulo           |
            ModuloAssign     |
            Return           |
            PowerOf          |
            PowerOfAssign => true,
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_bin_op(&self) -> bool {
        use self::Token::*;

        match *self {
            RCurly | RParen | RSquare | Plus | Minus | Multiply | Divide | Comma | Period
            | Equals | LessThan | GreaterThan | LessThanEqual | GreaterThanEqual | EqualTo
            | NotEqualTo | Pipe | Or | Ampersand | And | PowerOf => true,
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_un_op(&self) -> bool {
        use self::Token::*;

        match *self {
            UnaryPlus | UnaryMinus | Equals | Bang | Return => true,
            _ => false,
        }
    }
}

pub struct TokenIterator<'a> {
    last: Token,
    char_stream: Peekable<Chars<'a>>,
}

impl<'a> TokenIterator<'a> {
    pub fn parse_string_const(&mut self, enclosing_char: char) -> Result<String, LexError> {
        let mut result = Vec::new();
        let mut escape = false;

        while let Some(nxt) = self.char_stream.next() {
            match nxt {
                '\\' if !escape => escape = true,
                '\\' if escape => {
                    escape = false;
                    result.push('\\');
                }
                't' if escape => {
                    escape = false;
                    result.push('\t');
                }
                'n' if escape => {
                    escape = false;
                    result.push('\n');
                }
                'r' if escape => {
                    escape = false;
                    result.push('\r');
                }
                'x' if escape => {
                    escape = false;
                    let mut out_val: u32 = 0;
                    for _ in 0..2 {
                        if let Some(c) = self.char_stream.next() {
                            if let Some(d1) = c.to_digit(16) {
                                out_val *= 16;
                                out_val += d1;
                            } else {
                                return Err(LexError::MalformedEscapeSequence);
                            }
                        } else {
                            return Err(LexError::MalformedEscapeSequence);
                        }
                    }

                    if let Some(r) = char::from_u32(out_val) {
                        result.push(r);
                    } else {
                        return Err(LexError::MalformedEscapeSequence);
                    }
                }
                'u' if escape => {
                    escape = false;
                    let mut out_val: u32 = 0;
                    for _ in 0..4 {
                        if let Some(c) = self.char_stream.next() {
                            if let Some(d1) = c.to_digit(16) {
                                out_val *= 16;
                                out_val += d1;
                            } else {
                                return Err(LexError::MalformedEscapeSequence);
                            }
                        } else {
                            return Err(LexError::MalformedEscapeSequence);
                        }
                    }

                    if let Some(r) = char::from_u32(out_val) {
                        result.push(r);
                    } else {
                        return Err(LexError::MalformedEscapeSequence);
                    }
                }
                'U' if escape => {
                    escape = false;
                    let mut out_val: u32 = 0;
                    for _ in 0..8 {
                        if let Some(c) = self.char_stream.next() {
                            if let Some(d1) = c.to_digit(16) {
                                out_val *= 16;
                                out_val += d1;
                            } else {
                                return Err(LexError::MalformedEscapeSequence);
                            }
                        } else {
                            return Err(LexError::MalformedEscapeSequence);
                        }
                    }

                    if let Some(r) = char::from_u32(out_val) {
                        result.push(r);
                    } else {
                        return Err(LexError::MalformedEscapeSequence);
                    }
                }
                x if enclosing_char == x && escape => result.push(x),
                x if enclosing_char == x && !escape => break,
                _ if escape => return Err(LexError::MalformedEscapeSequence),
                _ => {
                    escape = false;
                    result.push(nxt);
                }
            }
        }

        let out: String = result.iter().cloned().collect();
        Ok(out)
    }

    fn inner_next(&mut self) -> Option<Token> {
        while let Some(c) = self.char_stream.next() {
            match c {
                '0'..='9' => {
                    let mut result = Vec::new();
                    let mut radix_base: Option<u32> = None;
                    result.push(c);

                    while let Some(&nxt) = self.char_stream.peek() {
                        match nxt {
                            '0'..='9' => {
                                result.push(nxt);
                                self.char_stream.next();
                            }
                            '.' => {
                                result.push(nxt);
                                self.char_stream.next();
                                while let Some(&nxt_float) = self.char_stream.peek() {
                                    match nxt_float {
                                        '0'..='9' => {
                                            result.push(nxt_float);
                                            self.char_stream.next();
                                        }
                                        _ => break,
                                    }
                                }
                            }
                            'x' | 'X' => {
                                result.push(nxt);
                                self.char_stream.next();
                                while let Some(&nxt_hex) = self.char_stream.peek() {
                                    match nxt_hex {
                                        '0'..='9' | 'a'..='f' | 'A'..='F' => {
                                            result.push(nxt_hex);
                                            self.char_stream.next();
                                        }
                                        _ => break,
                                    }
                                }
                                radix_base = Some(16);
                            }
                            'o' | 'O' => {
                                result.push(nxt);
                                self.char_stream.next();
                                while let Some(&nxt_oct) = self.char_stream.peek() {
                                    match nxt_oct {
                                        '0'..='8' => {
                                            result.push(nxt_oct);
                                            self.char_stream.next();
                                        }
                                        _ => break,
                                    }
                                }
                                radix_base = Some(8);
                            }
                            'b' | 'B' => {
                                result.push(nxt);
                                self.char_stream.next();
                                while let Some(&nxt_bin) = self.char_stream.peek() {
                                    match nxt_bin {
                                        '0' | '1' | '_' => {
                                            result.push(nxt_bin);
                                            self.char_stream.next();
                                        }
                                        _ => break,
                                    }
                                }
                                radix_base = Some(2);
                            }
                            _ => break,
                        }
                    }

                    if let Some(radix) = radix_base {
                        let out: String = result
                            .iter()
                            .cloned()
                            .skip(2)
                            .filter(|c| c != &'_')
                            .collect();
                        if let Ok(val) = i64::from_str_radix(&out, radix) {
                            return Some(Token::IntConst(val));
                        }
                    }

                    let out: String = result.iter().cloned().collect();

                    if let Ok(val) = out.parse::<i64>() {
                        return Some(Token::IntConst(val));
                    } else if let Ok(val) = out.parse::<f64>() {
                        return Some(Token::FloatConst(val));
                    }
                    return Some(Token::LexErr(LexError::MalformedNumber));
                }

                'A'..='Z' | 'a'..='z' | '_' => {
                    let mut result = Vec::new();
                    result.push(c);

                    while let Some(&nxt) = self.char_stream.peek() {
                        match nxt {
                            x if x.is_alphanumeric() || x == '_' => {
                                result.push(x);
                                self.char_stream.next();
                            }
                            _ => break,
                        }
                    }

                    let out: String = result.iter().cloned().collect();
                    match out.as_ref() {
                        "true" => return Some(Token::True),
                        "false" => return Some(Token::False),
                        "var" => return Some(Token::Var),
                        "if" => return Some(Token::If),
                        "end" => return Some(Token::End),
                        "else" => return Some(Token::Else),
                        "for" => return Some(Token::For),
                        "while" => return Some(Token::While),
                        "loop" => return Some(Token::Loop),
                        "break" => return Some(Token::Break),
                        "return" => return Some(Token::Return),
                        "new" => return Some(Token::New),
                        "func" => return Some(Token::Fn),
                        "ref" => return Some(Token::Ref),
                        "null" => return Some(Token::Null),
                        "cast" => return Some(Token::Cast),
                        "enum" => return Some(Token::Enum),
                        "this" => return Some(Token::This),
                        "class" => return Some(Token::Class),
                        "label" => return Some(Token::Label),
                        "goto" => return Some(Token::Goto),
                        x => {
                            return Some(Token::Identifier(x.into()));
                        }
                    }
                }
                '"' => match self.parse_string_const('"') {
                    Ok(out) => return Some(Token::StringConst(out)),
                    Err(e) => return Some(Token::LexErr(e)),
                },
                '\'' => match self.parse_string_const('\'') {
                    Ok(result) => {
                        let mut chars = result.chars();

                        if let Some(out) = chars.next() {
                            println!("result: {}", result);
                            if chars.count() != 0 {
                                return Some(Token::LexErr(LexError::MalformedChar));
                            }
                            return Some(Token::CharConst(out));
                        } else {
                            return Some(Token::LexErr(LexError::MalformedChar));
                        }
                    }
                    Err(e) => return Some(Token::LexErr(e)),
                },
                '{' => return Some(Token::LCurly),
                '}' => return Some(Token::RCurly),
                '(' => return Some(Token::LParen),
                ')' => return Some(Token::RParen),
                '[' => return Some(Token::LSquare),
                ']' => return Some(Token::RSquare),
                '+' => {
                    return match self.char_stream.peek() {
                        Some(&'=') => {
                            self.char_stream.next();
                            Some(Token::PlusAssign)
                        }
                        _ if self.last.is_next_unary() => Some(Token::UnaryPlus),
                        _ => Some(Token::Plus),
                    }
                }
                '-' => {
                    return match self.char_stream.peek() {
                        Some(&'=') => {
                            self.char_stream.next();
                            Some(Token::MinusAssign)
                        }
                        _ if self.last.is_next_unary() => Some(Token::UnaryMinus),
                        _ => Some(Token::Minus),
                    }
                }
                '*' => {
                    return match self.char_stream.peek() {
                        Some(&'=') => {
                            self.char_stream.next();
                            Some(Token::MultiplyAssign)
                        }
                        _ => Some(Token::Multiply),
                    }
                }
                '/' => match self.char_stream.peek() {
                    Some(&'/') => {
                        self.char_stream.next();
                        while let Some(c) = self.char_stream.next() {
                            if c == '\n' {
                                break;
                            }
                        }
                    }
                    Some(&'*') => {
                        let mut level = 1;
                        self.char_stream.next();
                        while let Some(c) = self.char_stream.next() {
                            match c {
                                '/' => {
                                    if let Some('*') = self.char_stream.next() {
                                        level += 1;
                                    }
                                }
                                '*' => {
                                    if let Some('/') = self.char_stream.next() {
                                        level -= 1;
                                    }
                                }
                                _ => (),
                            }

                            if level == 0 {
                                break;
                            }
                        }
                    }
                    Some(&'=') => {
                        self.char_stream.next();
                        return Some(Token::DivideAssign);
                    }
                    _ => return Some(Token::Divide),
                },
                ';' => return Some(Token::Semicolon),
                ':' => return Some(Token::Colon),
                ',' => return Some(Token::Comma),
                '.' => return Some(Token::Period),
                '=' => match self.char_stream.peek() {
                    Some(&'=') => {
                        self.char_stream.next();
                        return Some(Token::EqualTo);
                    }
                    _ => return Some(Token::Equals),
                },
                '<' => match self.char_stream.peek() {
                    Some(&'=') => {
                        self.char_stream.next();
                        return Some(Token::LessThanEqual);
                    }
                    Some(&'<') => {
                        self.char_stream.next();
                        return match self.char_stream.peek() {
                            Some(&'=') => {
                                self.char_stream.next();
                                Some(Token::LeftShiftAssign)
                            }
                            _ => {
                                self.char_stream.next();
                                Some(Token::LeftShift)
                            }
                        };
                    }
                    _ => return Some(Token::LessThan),
                },
                '>' => match self.char_stream.peek() {
                    Some(&'=') => {
                        self.char_stream.next();
                        return Some(Token::GreaterThanEqual);
                    }
                    Some(&'>') => {
                        self.char_stream.next();
                        return match self.char_stream.peek() {
                            Some(&'=') => {
                                self.char_stream.next();
                                Some(Token::RightShiftAssign)
                            }
                            _ => {
                                self.char_stream.next();
                                Some(Token::RightShift)
                            }
                        };
                    }
                    _ => return Some(Token::GreaterThan),
                },
                '!' => match self.char_stream.peek() {
                    Some(&'=') => {
                        self.char_stream.next();
                        return Some(Token::NotEqualTo);
                    }
                    _ => return Some(Token::Bang),
                },
                '|' => match self.char_stream.peek() {
                    Some(&'|') => {
                        self.char_stream.next();
                        return Some(Token::Or);
                    }
                    Some(&'=') => {
                        self.char_stream.next();
                        return Some(Token::OrAssign);
                    }
                    _ => return Some(Token::Pipe),
                },

                '&' => match self.char_stream.peek() {
                    Some(&'&') => {
                        self.char_stream.next();
                        return Some(Token::And);
                    }
                    Some(&'=') => {
                        self.char_stream.next();
                        return Some(Token::AndAssign);
                    }
                    _ => return Some(Token::Ampersand),
                },
                '^' => match self.char_stream.peek() {
                    Some(&'=') => {
                        self.char_stream.next();
                        return Some(Token::XOrAssign);
                    }
                    _ => return Some(Token::XOr),
                },
                '%' => match self.char_stream.peek() {
                    Some(&'=') => {
                        self.char_stream.next();
                        return Some(Token::ModuloAssign);
                    }
                    _ => return Some(Token::Modulo),
                },
                '~' => match self.char_stream.peek() {
                    Some(&'=') => {
                        self.char_stream.next();
                        return Some(Token::PowerOfAssign);
                    }
                    _ => return Some(Token::PowerOf),
                },

                _x if _x.is_whitespace() => (),
                _ => return Some(Token::LexErr(LexError::UnexpectedChar)),
            }
        }

        None
    }
}

impl<'a> Iterator for TokenIterator<'a> {
    type Item = Token;

    // TODO - perhaps this could be optimized?
    fn next(&mut self) -> Option<Self::Item> {
        self.last = match self.inner_next() {
            Some(c) => c,
            None => return None,
        };
        Some(self.last.clone())
    }
}

pub fn lex(input: &str) -> TokenIterator<'_> {
    TokenIterator {
        last: Token::LexErr(LexError::Nothing),
        char_stream: input.chars().peekable(),
    }
}

fn get_precedence(token: &Token) -> i32 {
    match *token {
        Token::Equals
        | Token::PlusAssign
        | Token::MinusAssign
        | Token::MultiplyAssign
        | Token::DivideAssign
        | Token::LeftShiftAssign
        | Token::RightShiftAssign
        | Token::AndAssign
        | Token::OrAssign
        | Token::XOrAssign
        | Token::ModuloAssign
        | Token::PowerOfAssign => 10,
        Token::Or | Token::XOr | Token::Pipe => 11,
        Token::And | Token::Ampersand => 12,
        Token::LessThan
        | Token::LessThanEqual
        | Token::GreaterThan
        | Token::GreaterThanEqual
        | Token::EqualTo
        | Token::NotEqualTo => 15,
        Token::Plus | Token::Minus => 20,

        Token::Divide | Token::Multiply | Token::PowerOf => 40,
        Token::LeftShift | Token::RightShift => 50,
        Token::Modulo => 60,
        //Token::Isa => 70,
        Token::Period => 100,

        _ => -1,
    }
}

fn parse_paren_expr<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Expr, ParseError> {
    let expr = parse_expr(input).unwrap();

    match input.next() {
        Some(Token::RParen) => Ok(expr),
        _ => Err(ParseError::MissingRParen),
    }
}

fn parse_new_expr<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Expr, ParseError> {
    let mut args = Vec::new();

    let name = match input.next() {
        Some(Token::Identifier(ref s)) => s.clone(),
        v => panic!("{:?}", v),
    };

    match input.peek() {
        Some(&Token::LParen) => {
            input.next();
        }
        _ => return Err(ParseError::MissingLParen),
    }

    if let Some(&Token::RParen) = input.peek() {
        input.next();
        return Ok(Expr::New(name, args));
    }

    loop {
        if let Ok(arg) = parse_expr(input) {
            args.push(arg);
        } else {
            return Err(ParseError::MalformedCallExpr);
        }

        match input.peek() {
            Some(&Token::RParen) => {
                input.next();
                return Ok(Expr::New(name, args));
            }
            Some(&Token::Comma) => (),
            _ => return Err(ParseError::MalformedCallExpr),
        }
        input.next();
    }
}

fn parse_call_expr<'a>(
    id: String,
    input: &mut Peekable<TokenIterator<'a>>,
) -> Result<Expr, ParseError> {
    let mut args = Vec::new();

    if let Some(&Token::RParen) = input.peek() {
        input.next();
        return Ok(Expr::FnCall(id, args));
    }

    loop {
        if let Ok(arg) = parse_expr(input) {
            args.push(arg);
        } else {
            return Err(ParseError::MalformedCallExpr);
        }

        match input.peek() {
            Some(&Token::RParen) => {
                input.next();
                return Ok(Expr::FnCall(id, args));
            }
            Some(&Token::Comma) => (),
            v => {
                println!("err {:?}", v);
                return Err(ParseError::MalformedCallExpr);
            }
        }

        input.next();
    }
}

fn parse_index_expr<'a>(
    id: String,
    input: &mut Peekable<TokenIterator<'a>>,
) -> Result<Expr, ParseError> {
    if let Ok(idx) = parse_expr(input) {
        match input.peek() {
            Some(&Token::RSquare) => {
                input.next();
                return Ok(Expr::Index(id, Box::new(idx)));
            }
            _ => return Err(ParseError::MalformedIndexExpr),
        }
    } else {
        return Err(ParseError::MalformedIndexExpr);
    }
}

fn parse_ident_expr<'a>(
    id: String,
    input: &mut Peekable<TokenIterator<'a>>,
    rf: bool,
) -> Result<Expr, ParseError> {
    match input.peek() {
        Some(&Token::LParen) => {
            input.next();
            parse_call_expr(id, input)
        }
        Some(&Token::LSquare) => {
            input.next();
            parse_index_expr(id, input)
        }
        _ => {
            if rf {
                Ok(Expr::RefIdentifier(id.clone()))
            } else {
                Ok(Expr::Identifier(id.clone()))
            }
        }
    }
}

fn parse_array_expr<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Expr, ParseError> {
    let mut arr = Vec::new();

    let skip_contents = match input.peek() {
        Some(&Token::RSquare) => true,
        _ => false,
    };

    if !skip_contents {
        while let Some(_) = input.peek() {
            arr.push(parse_expr(input)?);
            if let Some(&Token::Comma) = input.peek() {
                input.next();
            }

            if let Some(&Token::RSquare) = input.peek() {
                break;
            }
        }
    }

    match input.peek() {
        Some(&Token::RSquare) => {
            input.next();
            Ok(Expr::Array(arr))
        }
        _ => Err(ParseError::MissingRSquare),
    }
}

fn parse_primary<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Expr, ParseError> {
    if let Some(token) = input.next() {
        match token {
            Token::Cast => {
                match input.next() {
                    Some(Token::LessThan) => {
                        let ts = parse_type(input)?;
                        match input.next() {
                            Some(Token::GreaterThan) => {},
                            v => panic!("Unexpected {:?}",v)
                        }
                        let expr = parse_expr(input)?;
                        
                        Ok(Expr::Cast(ts,Box::new(expr)))
                    }
                    _ => panic!("Cast expression expects `<`"),
                }
                
            }
            Token::IntConst(ref x) => Ok(Expr::IntConst(*x)),
            Token::FloatConst(ref x) => Ok(Expr::FloatConst(*x)),
            Token::StringConst(ref s) => Ok(Expr::StringConst(s.clone())),
            Token::CharConst(ref c) => Ok(Expr::CharConst(*c)),
            Token::Identifier(ref s) => parse_ident_expr(s.clone(), input, false),
            Token::Ref => match input.next() {
                Some(Token::Identifier(ref id)) => parse_ident_expr(id.clone(), input, true),
                None => Err(ParseError::VarExpectsIdentifier),
                _ => Err(ParseError::VarExpectsIdentifier),
            },
            Token::New => parse_new_expr(input),
            Token::Null => Ok(Expr::Unit),
            Token::LParen => parse_paren_expr(input),
            Token::LSquare => parse_array_expr(input),
            Token::True => Ok(Expr::True),
            Token::False => Ok(Expr::False),
            Token::This => Ok(Expr::This),
            Token::LexErr(le) => {
                println!("Error: {}", le);
                Err(ParseError::BadInput)
            }
            _ => {
                println!("Can't parse: {:?}", token);
                Err(ParseError::BadInput)
            }
        }
    } else {
        Err(ParseError::InputPastEndOfFile)
    }
}

fn parse_unary<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Expr, ParseError> {
    let tok = match input.peek() {
        Some(tok) => tok.clone(),
        None => return Err(ParseError::InputPastEndOfFile),
    };

    match tok {
        Token::UnaryMinus => {
            input.next();
            Ok(Expr::FnCall(
                "__unary_minus__".into(),
                vec![parse_primary(input)?],
            ))
        }
        Token::UnaryPlus => {
            input.next();
            parse_primary(input)
        }
        Token::Bang => {
            input.next();
            Ok(Expr::Op(
                Op::Not,
                Box::new(parse_primary(input)?),
                Box::new(Expr::Unit),
            ))
        }
        _ => parse_primary(input),
    }
}

fn parse_binop<'a>(
    input: &mut Peekable<TokenIterator<'a>>,
    prec: i32,
    lhs: Expr,
) -> Result<Expr, ParseError> {
    let mut lhs_curr = lhs;

    loop {
        let mut curr_prec = -1;

        if let Some(curr_op) = input.peek() {
            curr_prec = get_precedence(curr_op);
        }

        if curr_prec < prec {
            return Ok(lhs_curr);
        }

        if let Some(op_token) = input.next() {
            let mut rhs = parse_unary(input)?;

            let mut next_prec = -1;

            if let Some(next_op) = input.peek() {
                next_prec = get_precedence(next_op);
            }

            if curr_prec < next_prec {
                rhs = parse_binop(input, curr_prec + 1, rhs)?;
            } else if curr_prec >= 100 {
                // Always bind right to left for precedence over 100
                rhs = parse_binop(input, curr_prec, rhs)?;
            }

            lhs_curr = match op_token {
                Token::Plus => Expr::Op(Op::Add, Box::new(lhs_curr), Box::new(rhs)),
                Token::Minus => Expr::Op(Op::Sub, Box::new(lhs_curr), Box::new(rhs)),
                Token::Multiply => Expr::Op(Op::Mul, Box::new(lhs_curr), Box::new(rhs)),
                Token::Divide => Expr::Op(Op::Div, Box::new(lhs_curr), Box::new(rhs)),
                Token::EqualTo => Expr::Op(Op::Eq, Box::new(lhs_curr), Box::new(rhs)),
                Token::GreaterThan => Expr::Op(Op::Gt, Box::new(lhs_curr), Box::new(rhs)),
                Token::LessThan => Expr::Op(Op::Lt, Box::new(lhs_curr), Box::new(rhs)),
                Token::Equals => Expr::Assignment(Box::new(lhs_curr), Box::new(rhs)),
                Token::And => Expr::Op(Op::And, Box::new(lhs_curr), Box::new(rhs)),
                Token::Or => Expr::Op(Op::Or, Box::new(lhs_curr), Box::new(rhs)),
                Token::Ampersand => Expr::Op(Op::BitAnd, Box::new(lhs_curr), Box::new(rhs)),
                Token::Pipe => Expr::Op(Op::BitOr, Box::new(lhs_curr), Box::new(rhs)),
                Token::XOr => Expr::Op(Op::BitXor, Box::new(lhs_curr), Box::new(rhs)),
                Token::LeftShift => Expr::Op(Op::Shl, Box::new(lhs_curr), Box::new(rhs)),
                Token::RightShift => Expr::Op(Op::Shr, Box::new(lhs_curr), Box::new(rhs)),
                Token::Period => Expr::Op(Op::Access, Box::new(lhs_curr), Box::new(rhs)),
                Token::LessThanEqual => Expr::Op(Op::Le, Box::new(lhs_curr), Box::new(rhs)),
                Token::PlusAssign => {
                    let lhs_copy = lhs_curr.clone();

                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::Op(Op::Add, Box::new(lhs_copy), Box::new(rhs))),
                    )
                }
                Token::MinusAssign => {
                    let lhs_copy = lhs_curr.clone();

                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::Op(Op::Sub, Box::new(lhs_copy), Box::new(rhs))),
                    )
                }
                Token::GreaterThanEqual => Expr::Op(Op::Ge, Box::new(lhs_curr), Box::new(rhs)),
                Token::PowerOf => Expr::Op(Op::Isa, Box::new(lhs_curr), Box::new(rhs)),
                Token::NotEqualTo => Expr::Op(Op::Ne, Box::new(lhs_curr), Box::new(rhs)),
                /* Token::PlusAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$+".to_string(), vec![lhs_copy, rhs])),
                    )
                }
                Token::MinusAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$-".to_string(), vec![lhs_copy, rhs])),
                    )
                }
                Token::Period => Expr::Dot(Box::new(lhs_curr), Box::new(rhs)),
                Token::EqualTo => Expr::FnCall("$==".to_string(), vec![lhs_curr, rhs]),
                Token::NotEqualTo => Expr::FnCall("$!=".to_string(), vec![lhs_curr, rhs]),
                Token::LessThan => Expr::FnCall("$<".to_string(), vec![lhs_curr, rhs]),
                Token::LessThanEqual => Expr::FnCall("$<=".to_string(), vec![lhs_curr, rhs]),
                Token::GreaterThan => Expr::FnCall("$>".to_string(), vec![lhs_curr, rhs]),
                Token::GreaterThanEqual => Expr::FnCall("$>=".to_string(), vec![lhs_curr, rhs]),
                Token::Or => Expr::FnCall("$||".to_string(), vec![lhs_curr, rhs]),
                Token::And => Expr::FnCall("$&&".to_string(), vec![lhs_curr, rhs]),
                Token::XOr => Expr::FnCall("$^".to_string(), vec![lhs_curr, rhs]),
                Token::OrAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$|".to_string(), vec![lhs_copy, rhs])),
                    )
                }
                Token::AndAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$&".to_string(), vec![lhs_copy, rhs])),
                    )
                }
                Token::XOrAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$^".to_string(), vec![lhs_copy, rhs])),
                    )
                }
                Token::MultiplyAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$*".to_string(), vec![lhs_copy, rhs])),
                    )
                }
                Token::DivideAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$/".to_string(), vec![lhs_copy, rhs])),
                    )
                }
                Token::Pipe => Expr::FnCall("$|".to_string(), vec![lhs_curr, rhs]),
                Token::LeftShift => Expr::FnCall("$<<".to_string(), vec![lhs_curr, rhs]),
                Token::RightShift => Expr::FnCall("$>>".to_string(), vec![lhs_curr, rhs]),
                Token::LeftShiftAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$<<".to_string(), vec![lhs_copy, rhs])),
                    )
                }
                Token::RightShiftAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$>>".to_string(), vec![lhs_copy, rhs])),
                    )
                }
                Token::Ampersand => Expr::FnCall("$&".to_string(), vec![lhs_curr, rhs]),
                Token::Modulo => Expr::FnCall("$%".to_string(), vec![lhs_curr, rhs]),
                Token::ModuloAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$%".to_string(), vec![lhs_copy, rhs])),
                    )
                }
                Token::PowerOf => Expr::FnCall("$~".to_string(), vec![lhs_curr, rhs]),
                Token::PowerOfAssign => {
                    let lhs_copy = lhs_curr.clone();
                    Expr::Assignment(
                        Box::new(lhs_curr),
                        Box::new(Expr::FnCall("$~".to_string(), vec![lhs_copy, rhs])),
                    )
                }*/
                v =>
                /*return Err(ParseError::UnknownOperator)*/
                {
                    panic!("Unknown operator {:?}", v)
                }
            };
        }
    }
}

fn parse_expr<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Expr, ParseError> {
    match input.peek() {
        Some(Token::RParen) => Ok(Expr::Unit),
        _ => {
            let lhs = parse_unary(input)?;

            parse_binop(input, 0, lhs)
        }
    }
}

fn parse_if<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Stmt, ParseError> {
    input.next();

    let guard = parse_expr(input)?;
    let body = parse_block(input)?;

    match input.peek() {
        Some(&Token::Else) => {
            input.next();
            let else_body = parse_block(input)?;
            Ok(Stmt::IfElse(
                Box::new(guard),
                Box::new(body),
                Box::new(else_body),
            ))
        }
        _ => Ok(Stmt::If(Box::new(guard), Box::new(body))),
    }
}

fn parse_for<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Stmt, ParseError> {
    input.next();
    match input.next() {
        Some(Token::LParen) => {}
        _ => return Err(ParseError::MissingLParen),
    }
    let value = parse_var(input)?;
    match input.next() {
        Some(Token::Semicolon) => {}
        _ => return Err(ParseError::MissingSemicolon),
    }
    let condition = parse_expr(input)?;
    match input.next() {
        Some(Token::Semicolon) => {}
        _ => return Err(ParseError::MissingSemicolon),
    }
    let expr = parse_expr(input)?;
    match input.next() {
        Some(Token::RParen) => {}
        _ => return Err(ParseError::MissingRParen),
    }
    let block = parse_block(input)?;

    Ok(Stmt::For(
        Box::new(value),
        Box::new(condition),
        Box::new(expr),
        Box::new(block),
    ))
}

fn parse_while<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Stmt, ParseError> {
    input.next();

    let guard = parse_expr(input)?;
    let body = parse_block(input)?;

    Ok(Stmt::While(Box::new(guard), Box::new(body)))
}

fn parse_loop<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Stmt, ParseError> {
    input.next();

    let body = parse_block(input)?;

    Ok(Stmt::Loop(Box::new(body)))
}

fn parse_label<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Stmt, ParseError> {
    input.next();
    let name = match input.next() {
        Some(Token::Identifier(ref s)) => s.clone(),
        _ => return Err(ParseError::VarExpectsIdentifier),
    };
    return Ok(Stmt::Label(name.clone()));
}

fn parse_var<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Stmt, ParseError> {
    input.next();

    let name = match input.next() {
        Some(Token::Identifier(ref s)) => s.clone(),
        _ => return Err(ParseError::VarExpectsIdentifier),
    };

    let ts = match input.peek() {
        Some(&Token::Colon) => {
            input.next();
            let ts = parse_type(input)?;
            ts
        }
        _ => return Err(ParseError::VarExpectsIdentifier),
    };

    match input.peek() {
        Some(&Token::Equals) => {
            input.next();
            let initializer = parse_expr(input)?;
            Ok(Stmt::Var(name, ts, Some(Box::new(initializer))))
        }
        _ => Ok(Stmt::Var(name, ts, None)),
    }
}

fn parse_block<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Stmt, ParseError> {
    match input.peek() {
        Some(&Token::LCurly) => (),
        Some(&Token::NewLine) => (),
        Some(&Token::Colon) => (),
        _ => return Err(ParseError::MissingLCurly),
    }

    input.next();

    let mut stmts = Vec::new();

    let skip_body = match input.peek() {
        Some(&Token::RCurly) => true,
        Some(&Token::End) => true,
        _ => false,
    };

    if !skip_body {
        while let Some(_) = input.peek() {
            stmts.push(parse_stmt(input)?);

            if let Some(&Token::Semicolon) = input.peek() {
                input.next();
            }

            if let Some(&Token::RCurly) = input.peek() {
                break;
            }
            if let Some(&Token::End) = input.peek() {
                break;
            }
        }
    }

    match input.peek() {
        Some(&Token::RCurly) => {
            input.next();
            Ok(Stmt::Block(stmts))
        }
        Some(&Token::End) => {
            input.next();
            Ok(Stmt::Block(stmts))
        }
        v => {
            println!("{:?}", v);
            Err(ParseError::MissingRCurly)
        }
    }
}

fn parse_expr_stmt<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Stmt, ParseError> {
    let expr = parse_expr(input)?;
    Ok(Stmt::Expr(Box::new(expr)))
}

fn parse_stmt<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Stmt, ParseError> {
    match input.peek() {
        Some(&Token::If) => parse_if(input),
        Some(&Token::While) => parse_while(input),
        Some(&Token::For) => parse_for(input),
        Some(&Token::Loop) => parse_loop(input),
        Some(&Token::Break) => {
            input.next();
            Ok(Stmt::Break)
        }
        Some(&Token::Return) => {
            input.next();
            match input.peek() {
                Some(&Token::Semicolon) => Ok(Stmt::Return),
                _ => {
                    let ret = parse_expr(input)?;
                    Ok(Stmt::ReturnWithVal(Box::new(ret)))
                }
            }
        }
        Some(&Token::Label) => parse_label(input),
        Some(&Token::Goto) => {
            input.next();
            match input.next() {
                Some(Token::Identifier(ref s)) => Ok(Stmt::Goto(s.clone())),
                _ => return Err(ParseError::VarExpectsIdentifier),
            }
        }
        Some(&Token::LCurly) => parse_block(input),
        Some(&Token::Var) => parse_var(input),
        _ => parse_expr_stmt(input),
    }
}

fn parse_class_block<'a>(
    input: &mut Peekable<TokenIterator<'a>>,
) -> Result<(Stmt, Vec<FnDef>), ParseError> {
    match input.peek() {
        Some(&Token::LCurly) => (),
        _ => return Err(ParseError::MissingLCurly),
    }

    input.next();

    let mut stmts = Vec::new();
    let mut fns = Vec::new();
    let skip_body = match input.peek() {
        Some(&Token::RCurly) => true,
        _ => false,
    };

    if !skip_body {
        while let Some(_) = input.peek() {
            if let Some(&Token::Var) = input.peek() {
                stmts.push(parse_var(input)?);
                if let Some(&Token::Semicolon) = input.peek() {
                    input.next();
                }
            }

            if let Some(&Token::Fn) = input.peek() {
                fns.push(parse_fn(input)?);
                if let Some(&Token::Semicolon) = input.peek() {
                    input.next();
                }
            }
            if let Some(&Token::Semicolon) = input.peek() {
                input.next();
            }

            if let Some(&Token::RCurly) = input.peek() {
                break;
            }
        }
    }

    match input.peek() {
        Some(&Token::RCurly) => {
            input.next();
            Ok((Stmt::Block(stmts), fns))
        }
        _ => Err(ParseError::MissingRCurly),
    }
}

fn parse_type<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<TypeSpec, ParseError> {
    match input.next() {
        Some(Token::Ref) => match input.next() {
            Some(Token::Identifier(tname)) => Ok(TypeSpec {
                name: tname.clone(),
                is_ref: true,
            }),
            v => {
                println!("{:?}", v);
                return Err(ParseError::VarExpectsIdentifier);
            }
        },
        Some(Token::Identifier(tname)) => Ok(TypeSpec {
            name: tname.clone(),
            is_ref: false,
        }),
        v => {
            println!("{:?}", v);
            return Err(ParseError::VarExpectsIdentifier);
        }
    }
}

fn parse_fn<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<FnDef, ParseError> {
    input.next();

    let name = match input.next() {
        Some(Token::Identifier(ref s)) => Box::new(Expr::Identifier(s.clone())),
        _ => return Err(ParseError::FnMissingName),
    };

    match input.peek() {
        Some(&Token::LParen) => {
            input.next();
        }
        _ => return Err(ParseError::FnMissingParams),
    }

    let mut params = HashMap::new();
    let mut params_ = Vec::new();

    let skip_params = match input.peek() {
        Some(&Token::RParen) => {
            input.next();
            true
        }
        _ => false,
    };

    if !skip_params {
        loop {
            match input.next() {
                Some(Token::RParen) => break,
                Some(Token::Comma) => (),
                Some(Token::Identifier(ref s)) => match input.next() {
                    Some(Token::Colon) => {
                        let ts = parse_type(input)?;
                        params.insert(s.clone(), ts);
                        params_.push(s.clone());
                    }
                    v => {
                        println!("{:?}", v);
                        return Err(ParseError::MalformedCallExpr);
                    }
                },
                v => {
                    println!("{:?}", v);
                    return Err(ParseError::MalformedCallExpr);
                }
            }
        }
    }

    match input.peek() {
        Some(Token::Colon) => input.next(),
        _ => panic!("Expected colon at return type"),
    };

    let ts = parse_type(input)?;

    let body = parse_block(input)?;

    Ok(FnDef {
        name: name,
        params: params_,
        body: Box::new(body),
        params_: params,
        ret_ty: ts,
    })
}

fn parse_top_level<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Vec<Global>, ParseError> {
    let mut globals = Vec::new();
    while let Some(_) = input.peek() {
        match input.peek() {
            //Some(&Token::Class) => globals.push(Global::ClassDefinition(parse_class(input)?)),
            Some(&Token::Fn) => globals.push(Global::FnDefenition(parse_fn(input)?)),
            Some(&Token::NewLine) => {}
            Some(&Token::Var) => globals.push(Global::Variable(parse_var(input)?)),
            _ => panic!("Expected top level element"),
        }

        if let Some(&Token::Semicolon) = input.peek() {
            input.next();
        }
    }

    Ok(globals)
}

pub fn parse<'a>(input: &mut Peekable<TokenIterator<'a>>) -> Result<Vec<Global>, ParseError> {
    parse_top_level(input)
}
