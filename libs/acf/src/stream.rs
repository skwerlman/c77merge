use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Read;

use tracing::instrument;

use crate::error::{AcfError, ParseError, Result, StreamError};
use crate::parse::AcfTokenStream;
use crate::token::AcfToken;

#[derive(Debug)]
pub struct StructuredAcfTokenStream<R: Read + Debug> {
    read: AcfTokenStream<R>,
    depth: i64,
}
impl<R: Read + Debug> Iterator for StructuredAcfTokenStream<R> {
    type Item = Result<AcfToken>;
    fn next(&mut self) -> Option<Result<AcfToken>> {
        self.try_next().transpose()
    }
}
impl<R: Read + Debug> StructuredAcfTokenStream<R> {
    #[instrument(level = "trace", ret)]
    pub fn new(read: AcfTokenStream<R>) -> Self {
        Self { read, depth: 0 }
    }

    #[instrument(level = "trace", ret)]
    pub fn depth(&self) -> i64 {
        self.depth
    }

    #[instrument(level = "trace", ret)]
    pub fn try_next(&mut self) -> Result<Option<AcfToken>> {
        let t = self.read.try_next()?;
        self.depth += match t {
            Some(AcfToken::DictStart) => 1,
            Some(AcfToken::DictEnd) => -1,
            _ => 0,
        };
        Ok(t)
    }

    #[instrument(level = "trace", ret)]
    pub fn expect_next(&mut self) -> Result<AcfToken> {
        self.try_next()?
            .ok_or(StreamError::from(ParseError::UnexpectedEof).into())
    }

    #[instrument(level = "trace", ret)]
    pub fn expect(&mut self, token: AcfToken) -> Result<()> {
        let t = self.expect_next()?;
        if t == token {
            Ok(())
        } else {
            Err(StreamError::UnexpectedToken { token: t }.into())
        }
    }

    #[instrument(level = "trace", skip(target), fields(target = target.as_ref()), ret)]
    pub fn select(&mut self, target: impl AsRef<str>) -> Result<Option<()>> {
        while let Some(t) = self.try_next()? {
            match t {
                AcfToken::String(field) => {
                    if field == target.as_ref() {
                        return Ok(Some(()));
                    }
                }
                AcfToken::DictEnd => {
                    break;
                }
                AcfToken::DictStart => {
                    self.close_dict()?;
                }
            }
        }
        Ok(None)
    }

    #[instrument(level = "trace", ret)]
    pub fn try_select_path(
        &mut self,
        path: impl IntoIterator<Item = impl AsRef<str>> + Debug,
    ) -> Result<Option<()>> {
        let mut not_first = false;
        for field in path {
            if not_first {
                self.expect(AcfToken::DictStart)?;
            } else {
                not_first = true;
            }
            if self.select(field)?.is_none() {
                return Ok(None);
            }
        }
        Ok(Some(()))
    }

    #[instrument(level = "trace", ret)]
    pub fn select_path<'a, T, U: AsRef<str>>(&mut self, path: &'a T) -> Result<()>
    where
        &'a T: IntoIterator<Item = U> + Debug,
    {
        self.try_select_path(path)?.ok_or_else(|| {
            let owned = path.into_iter().map(|s| s.as_ref().to_owned()).collect();
            StreamError::PathNotFound(owned).into()
        })
    }

    #[instrument(level = "trace", ret)]
    pub fn close_dict(&mut self) -> Result<()> {
        self.skip_to_depth(self.depth - 1)
    }

    #[instrument(level = "trace", ret)]
    fn skip_to_depth(&mut self, target_depth: i64) -> Result<()> {
        while self.depth > target_depth {
            self.expect_next()?;
        }
        Ok(())
    }
}

// impl<R: Read + Debug> TryFrom<StructuredAcfTokenStream<R>> for AcfHashMap {
//     type Error = AcfError;
//     fn try_from(mut stream: StructuredAcfTokenStream<R>) -> Result<Self> {
//         return stream.try_into()?;
//     }
// }

impl<'a, R: Read + Debug> TryFrom<&'a mut StructuredAcfTokenStream<R>> for AcfHashMap {
    type Error = AcfError;
    fn try_from(stream: &'a mut StructuredAcfTokenStream<R>) -> Result<Self> {
        let mut converted = HashMap::new();
        while let Some(key) = stream.try_next()? {
            match key {
                AcfToken::String(key_name) => match stream.try_next()? {
                    Some(AcfToken::String(val)) => {
                        converted.insert(key_name, val.into());
                    }
                    Some(AcfToken::DictStart) => {
                        converted.insert(
                            key_name,
                            <&mut StructuredAcfTokenStream<R> as std::convert::TryInto<
                                AcfHashMap,
                            >>::try_into(stream)?
                            .into(),
                        );
                    }
                    Some(AcfToken::DictEnd) => {
                        return Err(StreamError::UnexpectedToken {
                            token: AcfToken::DictEnd,
                        }
                        .into());
                    }
                    None => {
                        return Err(StreamError::Parse {
                            error: ParseError::UnexpectedEof,
                        }
                        .into());
                    }
                },
                AcfToken::DictEnd => break,
                tk => return Err(StreamError::UnexpectedToken { token: tk }.into()),
            }
        }

        Ok(converted.into())
    }
}

#[derive(Debug)]
pub struct AcfHashMap {
    map: HashMap<String, AcfHashMapValue>,
}

impl AcfHashMap {
    pub fn keys(&self) -> Vec<&String> {
        self.map.keys().collect()
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

impl From<HashMap<String, AcfHashMapValue>> for AcfHashMap {
    #[instrument(level = "trace", ret)]
    fn from(value: HashMap<String, AcfHashMapValue>) -> Self {
        AcfHashMap { map: value }
    }
}

impl From<AcfHashMap> for HashMap<String, AcfHashMapValue> {
    #[instrument(level = "trace", ret)]
    fn from(value: AcfHashMap) -> Self {
        value.map
    }
}

#[derive(Debug)]
pub enum AcfHashMapValue {
    Hash(AcfHashMap),
    String(String),
}

impl From<String> for AcfHashMapValue {
    #[instrument(level = "trace", ret)]
    fn from(value: String) -> Self {
        AcfHashMapValue::String(value)
    }
}

impl From<HashMap<String, AcfHashMapValue>> for AcfHashMapValue {
    #[instrument(level = "trace", ret)]
    fn from(value: HashMap<String, AcfHashMapValue>) -> Self {
        AcfHashMapValue::Hash(value.into())
    }
}

impl From<AcfHashMap> for AcfHashMapValue {
    #[instrument(level = "trace", ret)]
    fn from(value: AcfHashMap) -> Self {
        AcfHashMapValue::Hash(value)
    }
}
