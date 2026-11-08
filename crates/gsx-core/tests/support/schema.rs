use std::thread;

use boon::{Compiler, SchemaIndex, Schemas};
use serde_json::Value;

const SCHEMA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema/gsx-1.schema.json");
const STACK: usize = 32 << 20;

pub struct Schema {
    schemas: Schemas,
    index: SchemaIndex,
}

impl Schema {
    pub fn load() -> Schema {
        let mut schemas = Schemas::new();
        let index = Compiler::new().compile(SCHEMA, &mut schemas).unwrap_or_else(|error| panic!("{error:#}"));
        Schema { schemas, index }
    }

    pub fn check(&self, instance: &Value) -> Result<(), String> {
        self.schemas.validate(instance, self.index).map_err(|error| format!("{error}"))
    }

    pub fn check_wire(&self, out: &str) -> Result<(), String> {
        let instance = serde_json::from_str(out).map_err(|error| format!("not JSON ({error}): {out}"))?;
        self.check(&instance).map_err(|error| format!("{error}\n{out}"))
    }
}

pub fn deep<T: Send>(run: impl FnOnce() -> T + Send) -> T {
    thread::scope(|scope| thread::Builder::new().stack_size(STACK).spawn_scoped(scope, run).unwrap().join().unwrap())
}
