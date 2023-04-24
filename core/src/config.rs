use crate::HttpClient;

use crate::ProgressManager;

pub trait Config: 'static {
	type ProgressManager: ProgressManager;
	type HttpClient: HttpClient;
}
