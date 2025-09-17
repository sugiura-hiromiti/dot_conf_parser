use std::fmt::Debug;

use crate::ConfMap;
use crate::ConfValue;

pub trait Show: Debug {
	fn show(&self,) {
		self.show_as(ShowFmt::default(),);
	}
	fn show_as(&self, fmt: ShowFmt,);
}

impl Show for ConfMap {
	fn show_as(&self, fmt: ShowFmt,) {
		let output = match fmt {
			ShowFmt::Conf => conf_map_as_conf(self,),
			ShowFmt::Json => conf_map_as_json(self,),
			ShowFmt::Debug => conf_map_as_debug(self,),
		};

		println!("{output}")
	}
}

#[derive(Default,)]
pub enum ShowFmt {
	#[default]
	Conf,
	Json,
	Debug,
}

fn rec_conf_map_map<'a,>(
	conf_map: &'a ConfMap,
	on_scalar: impl Fn(&String, &String,) -> String + 'a,
	on_map: impl Fn(&String, &ConfMap,) -> String + 'a,
) -> impl Iterator<Item = String,> {
	conf_map.iter().map(move |(key, value,)| match value {
		ConfValue::Scalar(v,) => on_scalar(key, v,),
		ConfValue::Map(conf_map,) => on_map(key, conf_map,),
	},)
}

fn conf_map_as_conf(conf_map: &ConfMap,) -> String {
	fn conf_map_as_conf_inner(
		conf_map: &ConfMap,
	) -> impl Iterator<Item = String,> {
		rec_conf_map_map(
			conf_map,
			|k, v| k.to_owned() + " = " + v,
			|k, conf_map| {
				conf_map_as_conf_inner(conf_map,)
					.map(|s| k.to_owned() + "." + &s,)
					.collect::<Vec<_,>>()
					.join("\n",)
			},
		)
	}

	conf_map_as_conf_inner(conf_map,).collect::<Vec<_,>>().join("\n",)
}

fn conf_map_as_json(conf_map: &ConfMap,) -> String {
	fn value_stringify(conf_map: &ConfMap, indent: usize,) -> String {
		let indents = "\t".repeat(indent + 1,);
		let sep = ",\n".to_owned();
		let str_represent = rec_conf_map_map(
			conf_map,
			|k, v| indents.clone() + k + ": " + v,
			|k, conf_map| {
				let value = &value_stringify(conf_map, indent + 1,);
				indents.clone() + k + ": " + value
			},
		)
		.collect::<Vec<_,>>()
		.join(&sep,);

		"{\n".to_owned() + &str_represent + "\n" + &"\t".repeat(indent,) + "}"
	}

	// let dummy_key = "_";
	// let mut root = ConfMap::new();
	// root.insert(dummy_key.to_owned(), ConfValue::Map(conf_map.clone(),),);
	// let output = value_stringify(&root, 0,);

	value_stringify(conf_map, 0,)
	// output[dummy_key.len() + 2..].to_string()
}

fn conf_map_as_debug(conf_map: &ConfMap,) -> String {
	format!("{conf_map:#?}")
}

#[cfg(test)]
mod tests {
	use crate::error::ParseError;
	use crate::parse_str;

	use super::*;

	fn apply_to_parsed_result(
		input: &str,
		f: impl Fn(&ConfMap,) -> String,
	) -> Result<String, ParseError,> {
		let output = parse_str(input,)?;
		Ok(f(&output,),)
	}

	#[test]
	fn test_conf_map_as_conf() -> Result<(), ParseError,> {
		let input = r#"endpoint = localhost:3000
debug = true
log.file = /var/log/console.log
log.name = default.log
net.ipv4.ip_local_reserved_ports = 8080,9148
"#;
		let output = apply_to_parsed_result(input, conf_map_as_conf,)?;
		assert_eq!(
			r#"debug = true
endpoint = localhost:3000
log.file = /var/log/console.log
log.name = default.log
net.ipv4.ip_local_reserved_ports = 8080,9148"#,
			output
		);

		Ok((),)
	}

	#[test]
	fn test_conf_map_as_json() -> Result<(), ParseError,> {
		let input = "endpoint = localhost:3000\ndebug = true\nlog.file = \
		             /var/log/console.log\nlog.name = \
		             default.log\nnet.ipv4.ip_local_reserved_ports = \
		             8080,9148\n";
		let output = apply_to_parsed_result(input, conf_map_as_json,)?;
		eprintln!("{output}");
		assert_eq!(
			r#"{
	debug: true,
	endpoint: localhost:3000,
	log: {
		file: /var/log/console.log,
		name: default.log
	},
	net: {
		ipv4: {
			ip_local_reserved_ports: 8080,9148
		}
	}
}"#,
			output
		);

		Ok((),)
	}
}
