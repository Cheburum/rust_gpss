use std::fs;

macro_rules! get_keyword_name_impl {
     ($f_name: ident; $lex_type:ty; $lex_enum:ident; $($lexname:expr,$lexcode:ident),+) =>  {
                fn $f_name(code: $lex_enum) -> $lex_type{
                    match code{
                        $($lex_enum::$lexcode => $lexname.into(),)+
                    }
              }
    };
}

macro_rules! get_keyword_code_impl {
     ($f_name: ident; $lex_type:ty; $lex_enum:ident; $($lexname:expr,$lexcode:ident),+) =>  {
                fn $f_name(name: $lex_type) -> Option<$lex_enum>{
                    match name{
                        $($lexname => Some($lex_enum::$lexcode),)+
                        _ => None
                    }
              }
    };
}

macro_rules! tokens{
    ($tokens_type:ident; $($lex:ident),+) =>  {
                #[derive(Copy, Clone,Debug)]
                enum $tokens_type{$($lex),+}

    };
}

macro_rules! implement_lexer{
    // for tokens
    ($($lex:ident),+) => {
            tokens!(Keyword; $($lex),+);
            get_keyword_name_impl!(get_keyword_name; String; Keyword; $(stringify!($lex),$lex),+);
            get_keyword_code_impl!(get_keyword_code; &str; Keyword; $(stringify!($lex),$lex),+);
    };
    // for special symbols
    ($(|$name:expr,$code:ident|),+) =>{
            tokens!(Special; $($code),+);
            get_keyword_name_impl!(get_special_name; char; Special; $($name,$code),+);
            get_keyword_code_impl!(get_special_code; char; Special; $($name,$code),+);
    };
}

implement_lexer!(Generate, Terminate, Advance, Test, Seize, Release, Queue, Depart);
implement_lexer!(|' ',Space|, |'\t',Tab|, |'\n', Newline|,
                 |';',Semicolon|, |'\0', Endfile|,
                 |'/',Div|, |'*', Multiply|, |'+',Plus|, |'-',Minus|);

#[derive(Debug)]
pub enum Lexeme {
    Keyword(Keyword),
    Special(Special),
    UserIdentity(String),
}

pub fn lexer(filename: &str) -> Vec<Lexeme> {
    let buffer = fs::read_to_string(filename).unwrap();
    let mut ident = String::new();
    let mut lexems = Vec::new();
    let mut line_number: u32 = 1;

    for i in buffer.chars() {
        match get_special_code(i) {
            // Проверка, что сейчас спец-символ
            Some(special_code) => {
                //Увеличиваем счетчик линий
                match special_code {
                    Special::Newline => {
                        line_number += 1;
                    }
                    _ => {}
                };
                //Если сущность пустая, то пропускаем
                if ident.len() != 0 {
                    match get_keyword_code(ident.as_str()) {
                        //Если сущность является ключевым словом
                        Some(code) => {
                            lexems.push(Lexeme::Keyword(code));
                        }
                        // Если сущность не является ключевым словом
                        // Это может быть пользовательское название функции, переменной, блока
                        // или просто значение
                        None => {
                            println!(
                                "Lexer warning: unknown keyword: {}, line: {}",
                                ident, line_number
                            );

                            lexems.push(Lexeme::UserIdentity(ident.clone()));
                        }
                    };
                    ident.clear();
                }

                lexems.push(Lexeme::Special(special_code));
            }
            // Если не спец-символ, то продолжаем накопление символов
            None => {
                ident.push(i);
            }
        };
    }
    lexems
}