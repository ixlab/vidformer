use crate::dve::Error;
use crate::filter::*;
use std::collections::BTreeMap;
use std::io::Read;
use std::io::Write;

/// Filter over IPC Socket
///
/// This filter sends requests to a filter server over a Unix socket.
/// TODO: Wire protocol documentation
pub struct IPC {
    pub(crate) socket: String,
    pub(crate) func: String,
    pub(crate) state: parking_lot::Mutex<IPCFilterState>,
}

impl IPC {
    pub fn new(socket: String, func: String) -> Result<Self, Error> {
        let stream = std::os::unix::net::UnixStream::connect(&socket).map_err(|e| {
            Error::FilterInternalError(format!(
                "Failed to connect to IPC socket {}: {:?}",
                &socket, e
            ))
        })?;

        log::info!("Connected to IPC socket {}", &socket);
        Ok(Self {
            socket,
            func,
            state: parking_lot::Mutex::new(IPCFilterState { stream }),
        })
    }

    pub fn via_map(config: &BTreeMap<String, String>) -> Result<Self, Error> {
        let socket = config
            .get("socket")
            .ok_or_else(|| Error::Unknown("Missing 'socket' key".to_string()))?;

        let func = config
            .get("func")
            .ok_or_else(|| Error::Unknown("Missing 'func' key".to_string()))?;

        Self::new(socket.clone(), func.clone())
    }
}

pub(crate) struct IPCFilterState {
    pub(crate) stream: std::os::unix::net::UnixStream,
}

impl IPCFilterState {
    pub(crate) fn rpc(&mut self, req: &[u8]) -> Result<Vec<u8>, Error> {
        let req_len: u32 = req
            .len()
            .try_into()
            .map_err(|_| Error::FilterInternalError("IPC request too large".to_string()))?;
        self.stream.write_all(&req_len.to_be_bytes()).map_err(|e| {
            Error::FilterInternalError(format!("Failed to write IPC request length: {:?}", e))
        })?;
        self.stream.write_all(req).map_err(|e| {
            Error::FilterInternalError(format!("Failed to write IPC request: {:?}", e))
        })?;
        self.stream.flush().map_err(|e| {
            Error::FilterInternalError(format!("Failed to flush IPC request: {:?}", e))
        })?;

        let mut response_len_bytes = [0u8; 4];
        self.stream
            .read_exact(&mut response_len_bytes)
            .map_err(|e| {
                Error::FilterInternalError(format!("Failed to read IPC response length: {:?}", e))
            })?;
        let response_len = u32::from_be_bytes(response_len_bytes);
        let mut response_bytes = vec![0u8; response_len as usize];
        self.stream.read_exact(&mut response_bytes).map_err(|e| {
            Error::FilterInternalError(format!("Failed to read IPC response: {:?}", e))
        })?;
        Ok(response_bytes)
    }
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct IPCFilterRequest {
    pub(crate) func: String,
    pub(crate) op: &'static str,
    pub(crate) args: Vec<Val>,
    pub(crate) kwargs: BTreeMap<String, Val>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct IPCFilterResponse {
    pub(crate) frame: Frame,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct IPCFilterTypeRequest {
    pub(crate) func: String,
    pub(crate) op: &'static str,
    pub(crate) args: Vec<ValType>,
    pub(crate) kwargs: BTreeMap<String, ValType>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct IPCFilterTypeResponse {
    pub(crate) frame_type: FrameType,
}

impl crate::filter::Filter for IPC {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        let request = IPCFilterRequest {
            func: self.func.clone(),
            op: "filter",
            args: args.to_vec(),
            kwargs: kwargs.clone(),
        };
        let req_bytes = rmp_serde::to_vec_named(&request).map_err(|e| {
            Error::FilterInternalError(format!("Failed to serialize IPC request: {:?}", e))
        })?;

        let mut state = self.state.lock();
        let response_bytes = state.rpc(&req_bytes)?;
        drop(state);

        let response: IPCFilterResponse = rmp_serde::from_slice(&response_bytes).map_err(|e| {
            Error::FilterInternalError(format!(
                "Failed to deserialize filter IPC response from {}: {:?}",
                self.socket, e
            ))
        })?;

        Ok(response.frame)
    }

    fn filter_type(
        &self,
        args: &[ValType],
        kwargs: &BTreeMap<String, ValType>,
    ) -> Result<FrameType, Error> {
        for (i, arg) in args.iter().enumerate() {
            if let ValType::Frame(ft) = arg {
                if ft.format != ffi::AV_PIX_FMT_RGB24 {
                    return Err(Error::FilterInternalError(format!(
                        "Unsupported pixel format {} in argument {}",
                        crate::util::pixel_fmt_str(ft.format),
                        i
                    )));
                }
            }
        }

        for (key, val) in kwargs.iter() {
            if let ValType::Frame(ft) = val {
                if ft.format != ffi::AV_PIX_FMT_RGB24 {
                    return Err(Error::FilterInternalError(format!(
                        "Unsupported pixel format {:?} in keyword argument {}",
                        crate::util::pixel_fmt_str(ft.format),
                        key
                    )));
                }
            }
        }

        let request = IPCFilterTypeRequest {
            func: self.func.clone(),
            op: "filter_type",
            args: args.to_vec(),
            kwargs: kwargs.clone(),
        };
        let req_bytes = rmp_serde::to_vec_named(&request).map_err(|e| {
            Error::FilterInternalError(format!("Failed to serialize IPC request: {:?}", e))
        })?;

        let mut state: parking_lot::lock_api::MutexGuard<parking_lot::RawMutex, IPCFilterState> =
            self.state.lock();
        let response_bytes = state.rpc(&req_bytes)?;
        drop(state);

        let response: IPCFilterTypeResponse =
            rmp_serde::from_slice(&response_bytes).map_err(|e| {
                Error::FilterInternalError(format!(
                    "Failed to deserialize filter_type IPC response from {}: {:?}",
                    self.socket, e
                ))
            })?;

        Ok(response.frame_type)
    }
}
