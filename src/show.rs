use crate::parser::conf::ConfMap;
use crate::parser::conf::ConfValue;
use crate::parser::conf::SingleValue;
use crate::parser::conf::Value;
use std::fmt::Debug;

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

fn render_single(value: &SingleValue,) -> String {
	match value {
		SingleValue::String(s,) => s.clone(),
		SingleValue::Bool(flag,) => flag.to_string(),
		SingleValue::Integer(num,) => num.to_string(),
	}
}

fn render_scalar(value: &Value<SingleValue,>,) -> String {
	match value {
		Value::Single(inner,) => render_single(inner,),
		Value::Collection(entries,) => {
			entries.iter().map(render_single,).collect::<Vec<_,>>().join(",",)
		},
	}
}

fn conf_map_as_conf(conf_map: &ConfMap,) -> String {
	fn collect_entries(
		conf_map: &ConfMap,
		prefix: &str,
		output: &mut Vec<String,>,
	) {
		for (key, value,) in conf_map.iter() {
			match value {
				ConfValue::Scalar(scalar,) => {
					let full_key = if prefix.is_empty() {
						key.clone()
					} else {
						format!("{prefix}.{key}")
					};
					output.push(format!(
						"{full_key} = {}",
						render_scalar(scalar,),
					),);
				},
				ConfValue::Map(children,) => {
					let nested_prefix = if prefix.is_empty() {
						key.clone()
					} else {
						format!("{prefix}.{key}")
					};
					collect_entries(
						&ConfMap::from(children,),
						&nested_prefix,
						output,
					);
				},
			}
		}
	}

	let mut lines = Vec::new();
	collect_entries(conf_map, "", &mut lines,);
	lines.join("\n",)
}

fn conf_map_as_json(conf_map: &ConfMap,) -> String {
	fn render_map(conf_map: &ConfMap, indent: usize,) -> String {
		let indent_str = "\t".repeat(indent,);
		let child_indent = "\t".repeat(indent + 1,);
		let mut parts = Vec::new();

		for (key, value,) in conf_map.iter() {
			let rendered = match value {
				ConfValue::Scalar(scalar,) => {
					format!("{child_indent}{key}: {}", render_scalar(scalar,),)
				},
				ConfValue::Map(children,) => {
					let nested = ConfMap::from(children,);
					let nested_rendered = render_map(&nested, indent + 1,);
					format!("{child_indent}{key}: {nested_rendered}")
				},
			};
			parts.push(rendered,);
		}

		let body = parts.join(",\n",);
		format!("{{\n{body}\n{indent_str}}}")
	}

	render_map(conf_map, 0,)
}

fn conf_map_as_debug(conf_map: &ConfMap,) -> String {
	format!("{conf_map:#?}")
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::parser::conf::ConfValue;
	use crate::parser::conf::SingleValue;
	use crate::parser::conf::Value;

	fn sample_conf_map() -> ConfMap {
		let mut root = ConfMap::new();
		root.insert(
			"endpoint".to_string(),
			ConfValue::Scalar(Value::Single(SingleValue::String(
				"localhost:3000".to_string(),
			),),),
		);
		root.insert(
			"debug".to_string(),
			ConfValue::Scalar(Value::Single(SingleValue::Bool(true,),),),
		);
		let mut log_map = ConfMap::new();
		log_map.insert(
			"file".to_string(),
			ConfValue::Scalar(Value::Single(SingleValue::String(
				"/var/log/console.log".to_string(),
			),),),
		);
		log_map.insert(
			"name".to_string(),
			ConfValue::Scalar(Value::Single(SingleValue::String(
				"default.log".to_string(),
			),),),
		);
		root.insert("log".to_string(), ConfValue::Map(log_map.into_inner(),),);
		let mut net_map = ConfMap::new();
		let mut ipv4_map = ConfMap::new();
		ipv4_map.insert(
			"ip_local_reserved_ports".to_string(),
			ConfValue::Scalar(Value::Collection(vec![
				SingleValue::Integer(8080,),
				SingleValue::Integer(9148,),
			],),),
		);
		net_map
			.insert("ipv4".to_string(), ConfValue::Map(ipv4_map.into_inner(),),);
		root.insert("net".to_string(), ConfValue::Map(net_map.into_inner(),),);

		root
	}

	#[test]
	fn conf_map_as_conf_formats_entries() {
		let output = conf_map_as_conf(&sample_conf_map(),);
		assert_eq!(
			r"debug = true
endpoint = localhost:3000
log.file = /var/log/console.log
log.name = default.log
net.ipv4.ip_local_reserved_ports = 8080,9148",
			output
		);
	}

	#[test]
	fn conf_map_as_json_nested_structure() {
		let output = conf_map_as_json(&sample_conf_map(),);
		assert_eq!(
			r"{
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
}",
			output
		);
	}

	#[test]
	fn conf_map_as_debug_outputs_debug_string() {
		let mut conf_map = sample_conf_map();
		conf_map.insert(
			"feature.enabled".to_string(),
			ConfValue::Scalar(Value::Single(SingleValue::Bool(true,),),),
		);

		let debug = conf_map_as_debug(&conf_map,);
		assert!(debug.contains("feature.enabled"));
		assert!(debug.contains("true"));

		conf_map.show_as(ShowFmt::Debug,);
	}
}
