use crate::error::AppErrorMsg;

#[derive(Debug, Clone)]
pub struct FtpTaskResult<T> {
    pub result: Result<T, AppErrorMsg>,
    pub log: Vec<String>,
}

impl<T> FtpTaskResult<T> {
    pub fn from_response(result: Result<T, crate::error::AppError>, log: Vec<String>) -> Self {
        Self {
            result: result.map_err(Into::into),
            log,
        }
    }
}
