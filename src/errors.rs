use redis;

pub enum FromanError {
    RedisError(redis::RedisError)
}

impl From<redis::RedisError> for FromanError {
    fn from(error: redis::RedisError) -> Self {
        FromanError::RedisError(error)
    }
}

pub type FromanResult<T> = Result<T, FromanError>;
