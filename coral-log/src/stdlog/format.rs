use bytes::BufMut;
use prost::Message;

use super::record_proto;
use crate::error::CoralRes;

impl From<log::Level> for record_proto::Level {
    fn from(value: log::Level) -> Self {
        match value {
            log::Level::Error => Self::Error,
            log::Level::Warn => Self::Warn,
            log::Level::Info => Self::Info,
            log::Level::Debug => Self::Debug,
            log::Level::Trace => Self::Trace,
        }
    }
}

impl<'v> log::kv::VisitValue<'v> for record_proto::Field {
    fn visit_null(&mut self) -> Result<(), log::kv::Error> {
        self.val = "None".to_string();
        self.kind = record_proto::Kind::S.into();
        Ok(())
    }

    fn visit_u64(&mut self, value: u64) -> Result<(), log::kv::Error> {
        self.val = value.to_string();
        self.kind = record_proto::Kind::I.into();
        Ok(())
    }

    fn visit_i64(&mut self, value: i64) -> Result<(), log::kv::Error> {
        self.val = value.to_string();
        self.kind = record_proto::Kind::I.into();
        Ok(())
    }

    fn visit_u128(&mut self, value: u128) -> Result<(), log::kv::Error> {
        self.val = value.to_string();
        self.kind = record_proto::Kind::I.into();
        Ok(())
    }

    fn visit_i128(&mut self, value: i128) -> Result<(), log::kv::Error> {
        self.val = value.to_string();
        self.kind = record_proto::Kind::I.into();
        Ok(())
    }

    fn visit_f64(&mut self, value: f64) -> Result<(), log::kv::Error> {
        self.val = value.to_string();
        self.kind = record_proto::Kind::F.into();
        Ok(())
    }

    fn visit_bool(&mut self, value: bool) -> Result<(), log::kv::Error> {
        self.val = value.to_string();
        self.kind = record_proto::Kind::B.into();
        Ok(())
    }

    fn visit_str(&mut self, value: &str) -> Result<(), log::kv::Error> {
        self.visit_any(value.into())
    }

    fn visit_borrowed_str(&mut self, value: &'v str) -> Result<(), log::kv::Error> {
        self.visit_str(value)
    }

    fn visit_char(&mut self, value: char) -> Result<(), log::kv::Error> {
        let mut b = [0; 4];
        self.visit_str(&*value.encode_utf8(&mut b))
    }

    fn visit_any(&mut self, value: log::kv::Value) -> Result<(), log::kv::Error> {
        self.val = value.to_string();
        self.kind = record_proto::Kind::S.into();
        Ok(())
    }
}

impl<'kvs> log::kv::Visitor<'kvs> for record_proto::Record {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        let mut field = record_proto::Field::default();
        field.key = key.to_string();
        value.visit(&mut field)?;
        self.fields.push(field);
        Ok(())
    }
}

impl super::io::Convert for record_proto::Record {
    fn to_bytes(&mut self, record: &log::Record) -> CoralRes<Vec<u8>> {
        let current = std::thread::current();
        self.timestamp = chrono::Local::now().to_rfc3339();
        self.level = record_proto::Level::from(record.level()).into();
        if let Some(name) = current.name() {
            self.thread_name = name.to_owned();
        }
        if let Some(file) = record.file() {
            self.file = file.to_owned();
        }
        if let Some(line) = record.line() {
            self.line = line;
        }
        self.msg = record.args().to_string();
        let kvs = record.key_values();
        kvs.visit(self)?;
        let mut buf = bytes::BytesMut::with_capacity(1024);
        buf.put_u32(0);
        self.encode(&mut buf)?;
        let len_bytes = ((buf.len() - 4) as u32).to_be_bytes();
        buf[0] = len_bytes[0];
        buf[1] = len_bytes[1];
        buf[2] = len_bytes[2];
        buf[3] = len_bytes[3];
        Ok(buf.to_vec())
    }
}

#[cfg(test)]
mod test {
    use bytes::BufMut;
    use prost::Message;

    #[test]
    fn test_len() {
        let mut buf = bytes::BytesMut::with_capacity(1024);
        buf.put_u32(0);
        let mut record = super::record_proto::Record::default();
        record.thread_name = "thread name".to_string();
        record.encode(&mut buf).unwrap();
        let data = record.encode_to_vec();
        assert_eq!(buf.len() - 4, data.len());
        let len = ((buf.len() - 4) as u32).to_be_bytes();
        buf[0] = len[0];
        buf[1] = len[1];
        buf[2] = len[2];
        buf[3] = len[3];
        let b = buf.freeze();
        let len_bytes: [u8; 4] = b[..4].try_into().unwrap();
        let dec_len = u32::from_be_bytes(len_bytes) as usize;
        assert_eq!(dec_len, data.len());
        let dec_record = super::record_proto::Record::decode(&b[4..dec_len + 4]).unwrap();
        assert_eq!(dec_record.thread_name, "thread name");
    }
}
