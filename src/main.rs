use futures::task::{Context, Poll};
use nom::{
    branch::alt,
    bytes::streaming::tag,
    character::streaming::{alpha1, alphanumeric0, multispace0},
    combinator::recognize,
    sequence::tuple,
    Err,
    IResult,
};
use std::{convert::AsRef, pin::Pin};
use tokio::{
    io,
    io::{AsyncBufReadExt, AsyncWriteExt},
    stream::{Stream, StreamExt},
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Token {
    Lpar,
    Rpar,
    Sym(String),
}

pub fn token<'a>(input: &'a str) -> IResult<&'a str, Token> {
    let (i, _) = multispace0(input)?;
    alt((lpar, rpar, sym))(i)
}

pub fn lpar<'a>(input: &'a str) -> IResult<&'a str, Token> {
    let (i, _) = tag("(")(input)?;
    Ok((i, Token::Lpar))
}

pub fn rpar<'a>(input: &'a str) -> IResult<&'a str, Token> {
    let (i, _) = tag(")")(input)?;
    Ok((i, Token::Rpar))
}

pub fn sym<'a>(input: &'a str) -> IResult<&'a str, Token> {
    let (i, res) = recognize(tuple((alpha1, alphanumeric0)))(input)?;
    let res = String::from(res);
    Ok((i, Token::Sym(res)))
}

pub struct TokenStream<T, E, S>
where
    T: AsRef<str>,
    S: Stream<Item = Result<T, E>>,
{
    buffer: String,
    stream: S,
}

impl<T, E, S> TokenStream<T, E, S>
where
    T: AsRef<str>,
    S: Stream<Item = Result<T, E>>,
{
    pub fn new(stream: S) -> Self {
        let buffer = String::new();
        TokenStream { buffer, stream }
    }
}

impl<T, E, S> Stream for TokenStream<T, E, S>
where
    T: AsRef<str>,
    S: Stream<Item = Result<T, E>> + Unpin,
{
    type Item = Token;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Token>> {
        match token(self.buffer.as_ref()) {
            Ok((i, tok)) => {
                self.buffer = String::from(i);
                cx.waker().clone().wake();
                Poll::Ready(Some(tok))
            },
            Err(err) => match err {
                Err::Incomplete(needed) => {
                    println!("Err::Incomplete: {:?}", needed);
                    match Pin::new(&mut self.stream).poll_next(cx) {
                        Poll::Ready(None) => Poll::Ready(None),
                        Poll::Ready(Some(res)) => match res {
                            Ok(item) => {
                                println!("Stream::Ok: {:?}", item.as_ref());
                                self.buffer.push_str(item.as_ref());
                                cx.waker().clone().wake();
                                Poll::Pending
                            },
                            Err(_err) => {
                                println!("Stream::Err");
                                Poll::Ready(None)
                            },
                        },
                        Poll::Pending => Poll::Pending,
                    }
                },
                Err::Error(error) => {
                    println!("Err::Error: {:?}", error);
                    Poll::Ready(None)
                },
                Err::Failure(failure) => {
                    println!("Err::Failure: {:?}", failure);
                    Poll::Ready(None)
                },
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let stdin = io::stdin();
    let stdin = io::BufReader::new(stdin);
    let lines = stdin.lines();
    let mut tokens = TokenStream::new(lines);
    let mut stdout = io::stdout();
    while let Some(tok) = tokens.next().await {
        let out = format!("tok: {:?}\n", tok);
        stdout.write(&out.as_bytes()).await?;
    }
    Ok(())
}
