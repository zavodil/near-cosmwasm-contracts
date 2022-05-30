use crate::*;

pub fn is_expired(end_height: Option<BlockHeight>, end_time: Option<BlockHeight>) -> bool {
    if let Some(end_height) = end_height {
        if env::block_height() > end_height {
            return true;
        }
    }

    if let Some(end_time) = end_time {
        if env::block_timestamp() > end_time * 1000 {
            return true;
        }
    }
    false
}
