use super::record_proto;

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

impl<'kvs> log::kv::Visitor<'kvs> for record_proto::Field {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        self.key = key.to_string();
        value.visit(self);
        Ok(())
    }
}

impl log::kv::Source for record_proto::Record {
    fn visit<'kvs>(
        &'kvs self,
        visitor: &mut dyn log::kv::Visitor<'kvs>,
    ) -> Result<(), log::kv::Error> {
        // visitor.visit_pair(, )
        Ok(())
    }
}

impl super::io::Convert for record_proto::Record {
    fn into(&mut self, record: &log::Record) -> Vec<u8> {
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
        // record.key_values().visit(self);
        todo!()
    }
}
