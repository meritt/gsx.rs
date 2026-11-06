use serde_json::Value;

const BENCH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/bench");

#[derive(Debug)]
pub struct Case {
    pub id: String,
    pub group: String,
    source: Value,
}

#[derive(Debug)]
pub struct Bench {
    blocks: Value,
    pub cases: Vec<Case>,
}

pub fn typical() -> Vec<String> {
    read("typical.jsonl")
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str::<Value>(line).unwrap()["markdown"].as_str().unwrap().to_owned())
        .collect()
}

pub fn units(text: &str) -> usize {
    text.encode_utf16().count()
}

impl Bench {
    pub fn load() -> Bench {
        let spec: Value = serde_json::from_str(&read("cases.json")).unwrap();
        let cases = spec["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|case| Case {
                id: case["id"].as_str().unwrap().into(),
                group: case["group"].as_str().unwrap().into(),
                source: case["source"].clone(),
            })
            .collect();
        Bench { blocks: spec["blocks"].clone(), cases }
    }

    pub fn case(&self, id: &str) -> &Case {
        self.cases.iter().find(|case| case.id == id).unwrap()
    }

    pub fn markdown(&self, case: &Case, divisor: usize) -> String {
        self.expand(&case.source, divisor)
    }

    fn expand(&self, source: &Value, divisor: usize) -> String {
        let count = |key: &str| source.get(key).map(|count| (count.as_u64().unwrap() as usize / divisor).max(1));
        if let Some(text) = source.as_str() {
            return text.into();
        }
        if let Some(block) = source.get("block") {
            return self.blocks[block.as_str().unwrap()].as_str().unwrap().into();
        }
        if let Some(parts) = source.get("concat") {
            return parts.as_array().unwrap().iter().map(|part| self.expand(part, divisor)).collect();
        }
        if let Some(ramp) = source.get("ramp") {
            let (pre, step) = (ramp["pre"].as_str().unwrap(), ramp["step"].as_str().unwrap());
            let from = ramp["from"].as_u64().unwrap() as usize;
            return (0..count("times").unwrap()).flat_map(|i| [pre.to_owned(), step.repeat(from + i)]).collect();
        }
        let unit = self.expand(&source["repeat"], 1);
        if let Some(times) = count("times") {
            return unit.repeat(times);
        }
        let until = count("until").expect("repeat without times or until");
        unit.repeat((until / units(&unit)).max(1))
    }
}

fn read(file: &str) -> String {
    std::fs::read_to_string(format!("{BENCH}/{file}")).unwrap()
}
