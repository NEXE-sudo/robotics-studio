use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_EXPANDED_NODES: usize = 50_000;
const MAX_INCLUDE_FILES: usize = 32;
const MAX_DEPTH: usize = 32;
const MAX_EXPANSION_TIME: Duration = Duration::from_secs(2);

#[derive(Clone, Debug)]
struct XmlNode {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<XmlNode>,
    text: String,
}

impl XmlNode {
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value.as_str()))
    }
}

fn parse_document(content: &str, deadline: Instant) -> Result<XmlNode, String> {
    if content.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "Xacro source exceeds the {MAX_SOURCE_BYTES}-byte limit"
        ));
    }
    if content.contains('\0') {
        return Err("Xacro source contains a NUL character".to_string());
    }

    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut stack = Vec::<XmlNode>::new();
    let mut root = None;
    let mut node_count = 0usize;

    loop {
        if Instant::now() >= deadline {
            return Err("Xacro parsing exceeded its 2-second time limit".to_string());
        }
        match reader.read_event() {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                let (element, is_empty) = match event {
                    Event::Start(element) => (element, false),
                    Event::Empty(element) => (element, true),
                    _ => unreachable!(),
                };
                node_count += 1;
                if node_count > MAX_EXPANDED_NODES || stack.len() >= MAX_DEPTH {
                    return Err("Xacro XML exceeds the node or nesting limit".to_string());
                }
                let mut attributes = Vec::new();
                for attribute in element.attributes().with_checks(true) {
                    let attribute =
                        attribute.map_err(|error| format!("Malformed Xacro attribute: {error}"))?;
                    let name = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
                    let value = attribute
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|error| format!("Malformed Xacro attribute value: {error}"))?
                        .into_owned();
                    attributes.push((name, value));
                }
                let node = XmlNode {
                    name: String::from_utf8_lossy(element.name().as_ref()).into_owned(),
                    attributes,
                    children: Vec::new(),
                    text: String::new(),
                };
                if is_empty {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else if root.replace(node).is_some() {
                        return Err("Xacro document must have exactly one root element".to_string());
                    }
                } else {
                    stack.push(node);
                }
            }
            Ok(Event::End(element)) => {
                let node = stack
                    .pop()
                    .ok_or_else(|| "Xacro XML elements are improperly nested".to_string())?;
                if element.name().as_ref() != node.name.as_bytes() {
                    return Err("Xacro XML elements are improperly nested".to_string());
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else if root.replace(node).is_some() {
                    return Err("Xacro document must have exactly one root element".to_string());
                }
            }
            Ok(Event::Text(text)) => {
                if let Some(parent) = stack.last_mut() {
                    let decoded = text
                        .decode()
                        .map_err(|error| format!("Malformed Xacro text: {error}"))?;
                    parent.text.push_str(&decoded);
                } else {
                    let text_bytes: &[u8] = text.as_ref();
                    if !text_bytes.iter().all(u8::is_ascii_whitespace) {
                        return Err("Xacro contains text outside its root element".to_string());
                    }
                }
            }
            Ok(Event::CData(text)) => {
                if let Some(parent) = stack.last_mut() {
                    parent.text.push_str(
                        &text
                            .decode()
                            .map_err(|error| format!("Malformed Xacro text: {error}"))?,
                    );
                } else {
                    return Err("Xacro contains data outside its root element".to_string());
                }
            }
            Ok(Event::DocType(_)) => return Err("Xacro documents must not contain a DTD".into()),
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("Malformed Xacro XML: {error}")),
        }
    }

    if !stack.is_empty() {
        return Err("Xacro document is incomplete".to_string());
    }
    root.ok_or_else(|| "Xacro document has no root element".to_string())
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn is_xacro(node: &XmlNode, name: &str) -> bool {
    node.name.starts_with("xacro:") && local_name(&node.name) == name
}

#[derive(Clone)]
struct Context {
    properties: HashMap<String, String>,
    macros: HashMap<String, XmlNode>,
    include_stack: HashSet<PathBuf>,
    expanded_nodes: usize,
    expanded_bytes: usize,
    include_count: usize,
    deadline: Instant,
}

fn valid_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

struct ExpressionParser<'a> {
    source: &'a str,
    position: usize,
    depth: usize,
    properties: &'a HashMap<String, String>,
}

impl ExpressionParser<'_> {
    fn skip_whitespace(&mut self) {
        while self
            .source
            .as_bytes()
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn parse_sum(&mut self) -> Result<f64, String> {
        let mut value = self.parse_product()?;
        loop {
            self.skip_whitespace();
            match self.source.as_bytes().get(self.position) {
                Some(b'+') => {
                    self.position += 1;
                    value += self.parse_product()?;
                }
                Some(b'-') => {
                    self.position += 1;
                    value -= self.parse_product()?;
                }
                _ => return Ok(value),
            }
            if !value.is_finite() {
                return Err("Xacro numeric expression is not finite".to_string());
            }
        }
    }

    fn parse_product(&mut self) -> Result<f64, String> {
        let mut value = self.parse_atom()?;
        loop {
            self.skip_whitespace();
            match self.source.as_bytes().get(self.position) {
                Some(b'*') => {
                    self.position += 1;
                    value *= self.parse_atom()?;
                }
                Some(b'/') => {
                    self.position += 1;
                    let divisor = self.parse_atom()?;
                    if divisor == 0.0 {
                        return Err("Xacro numeric expression divides by zero".to_string());
                    }
                    value /= divisor;
                }
                _ => return Ok(value),
            }
            if !value.is_finite() {
                return Err("Xacro numeric expression is not finite".to_string());
            }
        }
    }

    fn parse_atom(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        let bytes = self.source.as_bytes();
        if bytes.get(self.position) == Some(&b'(') {
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return Err("Xacro numeric expression exceeds the nesting limit".to_string());
            }
            self.position += 1;
            let value = self.parse_sum()?;
            self.skip_whitespace();
            if bytes.get(self.position) != Some(&b')') {
                return Err("Xacro numeric expression has unbalanced parentheses".to_string());
            }
            self.position += 1;
            self.depth -= 1;
            return Ok(value);
        }
        if matches!(bytes.get(self.position), Some(b'+') | Some(b'-')) {
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return Err("Xacro numeric expression exceeds the nesting limit".to_string());
            }
            let negative = bytes[self.position] == b'-';
            self.position += 1;
            let value = self.parse_atom()?;
            self.depth -= 1;
            return Ok(if negative { -value } else { value });
        }
        let start = self.position;
        if bytes
            .get(self.position)
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        {
            self.position += 1;
            while bytes
                .get(self.position)
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            {
                self.position += 1;
            }
            let identifier = &self.source[start..self.position];
            if identifier == "pi" {
                return Ok(std::f64::consts::PI);
            }
            let value = self
                .properties
                .get(identifier)
                .ok_or_else(|| format!("Xacro property '{identifier}' is not defined"))?
                .parse::<f64>()
                .map_err(|_| {
                    format!("Xacro property '{identifier}' is not numeric in this expression")
                })?;
            return if value.is_finite() {
                Ok(value)
            } else {
                Err("Xacro numeric expression is not finite".to_string())
            };
        }

        while bytes
            .get(self.position)
            .is_some_and(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E'))
        {
            self.position += 1;
            if matches!(bytes[self.position - 1], b'e' | b'E')
                && matches!(bytes.get(self.position), Some(b'+') | Some(b'-'))
            {
                self.position += 1;
            }
        }
        if start == self.position {
            return Err("Unsupported Xacro numeric expression".to_string());
        }
        self.source[start..self.position]
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| "Invalid Xacro numeric expression".to_string())
    }
}

fn evaluate_expression(
    expression: &str,
    properties: &HashMap<String, String>,
) -> Result<f64, String> {
    let mut parser = ExpressionParser {
        source: expression,
        position: 0,
        depth: 0,
        properties,
    };
    let value = parser.parse_sum()?;
    parser.skip_whitespace();
    if parser.position != expression.len() {
        return Err("Unsupported Xacro numeric expression".to_string());
    }
    Ok(value)
}

fn substitute(source: &str, properties: &HashMap<String, String>) -> Result<String, String> {
    let mut output = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = ["${", "$(arg ", "$(find "]
        .iter()
        .filter_map(|needle| rest.find(needle))
        .min()
    {
        output.push_str(&rest[..start]);
        let is_arg = rest[start..].starts_with("$(arg ");
        let is_find = rest[start..].starts_with("$(find ");
        let (prefix_length, suffix) = if is_arg {
            (6, ')')
        } else if is_find {
            (7, ')')
        } else {
            (2, '}')
        };
        let value_start = start + prefix_length;
        let end = rest[value_start..]
            .find(suffix)
            .map(|index| value_start + index)
            .ok_or_else(|| "Xacro substitution is not terminated".to_string())?;
        let expression = rest[value_start..end].trim();
        if is_arg {
            if !valid_identifier(expression) {
                return Err(format!("Invalid Xacro argument '{expression}'"));
            }
            let value = properties
                .get(expression)
                .ok_or_else(|| format!("Xacro argument '{expression}' is not defined"))?;
            output.push_str(value);
        } else if is_find {
            if !valid_identifier(expression) {
                return Err(format!("Invalid Xacro package name '{expression}'"));
            }
            output.push_str("package://");
            output.push_str(expression);
        } else if valid_identifier(expression) {
            let value = properties.get(expression).ok_or_else(|| {
                format!("Xacro property or argument '{expression}' is not defined")
            })?;
            output.push_str(value);
        } else {
            output.push_str(&evaluate_expression(expression, properties)?.to_string());
        }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    if output.contains("${") || output.contains("$(") {
        return Err("Unsupported Xacro substitution syntax".to_string());
    }
    Ok(output)
}

fn parse_macro_params(value: &str) -> Result<Vec<(String, Option<String>)>, String> {
    value
        .split_whitespace()
        .map(|parameter| {
            if parameter.starts_with('*') || parameter.starts_with('^') {
                return Err("Xacro block parameters are not supported".to_string());
            }
            let (name, default) = parameter
                .split_once(":=")
                .map_or((parameter, None), |(name, default)| {
                    (name, Some(default.to_string()))
                });
            if !valid_identifier(name) {
                return Err(format!("Invalid Xacro macro parameter '{name}'"));
            }
            Ok((name.to_string(), default))
        })
        .collect()
}

fn condition(value: &str) -> Result<bool, String> {
    match value.trim() {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => value
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .map(|number| number != 0.0)
            .ok_or_else(|| {
                format!("Unsupported Xacro condition '{value}'; use a boolean or number")
            }),
    }
}

fn expand_nodes<F>(
    nodes: &[XmlNode],
    source_path: &Path,
    context: &mut Context,
    macro_depth: usize,
    include_loader: &mut F,
) -> Result<Vec<XmlNode>, String>
where
    F: FnMut(&Path, &str) -> Result<(PathBuf, String), String>,
{
    if macro_depth > MAX_DEPTH {
        return Err("Xacro macro recursion exceeds the nesting limit".to_string());
    }
    let mut output = Vec::new();
    for node in nodes {
        if Instant::now() >= context.deadline {
            return Err("Xacro expansion exceeded its 2-second time limit".to_string());
        }
        context.expanded_nodes += 1;
        if context.expanded_nodes > MAX_EXPANDED_NODES {
            return Err("Xacro expansion exceeds the node limit".to_string());
        }
        if is_xacro(node, "property") {
            let name = node
                .attribute("name")
                .ok_or_else(|| "Xacro property is missing its name".to_string())?;
            if !valid_identifier(name) {
                return Err(format!("Invalid Xacro property name '{name}'"));
            }
            let value = node
                .attribute("value")
                .ok_or_else(|| format!("Xacro property '{name}' is missing its value"))?;
            let value = substitute(value, &context.properties)?;
            if context.properties.len() >= 1024 && !context.properties.contains_key(name) {
                return Err("Xacro expansion exceeds the 1024-property limit".to_string());
            }
            context.properties.insert(name.to_string(), value);
            continue;
        }
        if is_xacro(node, "arg") {
            let name = node
                .attribute("name")
                .ok_or_else(|| "Xacro argument is missing its name".to_string())?;
            if !valid_identifier(name) {
                return Err(format!("Invalid Xacro argument name '{name}'"));
            }
            let default = node
                .attribute("default")
                .ok_or_else(|| format!("Xacro argument '{name}' needs a default value"))?;
            let value = substitute(default, &context.properties)?;
            context.properties.entry(name.to_string()).or_insert(value);
            continue;
        }
        if is_xacro(node, "macro") {
            let name = node
                .attribute("name")
                .ok_or_else(|| "Xacro macro is missing its name".to_string())?;
            if !valid_identifier(name) {
                return Err(format!("Invalid Xacro macro name '{name}'"));
            }
            context.macros.insert(name.to_string(), node.clone());
            continue;
        }
        if is_xacro(node, "include") {
            context.include_count += 1;
            if context.include_count > MAX_INCLUDE_FILES {
                return Err(format!(
                    "Xacro includes exceed the {MAX_INCLUDE_FILES}-file limit"
                ));
            }
            let filename = node
                .attribute("filename")
                .ok_or_else(|| "Xacro include is missing its filename".to_string())?;
            let filename = substitute(filename, &context.properties)?;
            let (included_path, included_content) = include_loader(source_path, &filename)?;
            if !context.include_stack.insert(included_path.clone()) {
                return Err("Xacro include cycle detected".to_string());
            }
            let included_root = parse_document(&included_content, context.deadline)?;
            if local_name(&included_root.name) != "robot" {
                return Err("Included Xacro file must have a <robot> root".to_string());
            }
            let included_children = expand_nodes(
                &included_root.children,
                &included_path,
                context,
                macro_depth,
                include_loader,
            )?;
            context.include_stack.remove(&included_path);
            output.extend(included_children);
            continue;
        }
        if is_xacro(node, "if") || is_xacro(node, "unless") {
            let value = node
                .attribute("value")
                .ok_or_else(|| "Xacro conditional is missing its value".to_string())?;
            let value = substitute(value, &context.properties)?;
            let mut include_children = condition(&value)?;
            if is_xacro(node, "unless") {
                include_children = !include_children;
            }
            if include_children {
                output.extend(expand_nodes(
                    &node.children,
                    source_path,
                    context,
                    macro_depth,
                    include_loader,
                )?);
            }
            continue;
        }
        if node.name.starts_with("xacro:") {
            let macro_name = local_name(&node.name);
            let definition =
                context.macros.get(macro_name).cloned().ok_or_else(|| {
                    format!("Unsupported or undefined Xacro element '{}'", node.name)
                })?;
            let params = parse_macro_params(
                definition
                    .attribute("params")
                    .ok_or_else(|| format!("Xacro macro '{macro_name}' has no params"))?,
            )?;
            let mut local_properties = context.properties.clone();
            for (name, default) in params {
                let value = if let Some(value) = node.attribute(&name) {
                    substitute(value, &context.properties)?
                } else if let Some(default) = default {
                    substitute(&default, &context.properties)?
                } else {
                    return Err(format!(
                        "Xacro macro '{macro_name}' is missing parameter '{name}'"
                    ));
                };
                local_properties.insert(name, value);
            }
            let previous = std::mem::replace(&mut context.properties, local_properties);
            let expanded = expand_nodes(
                &definition.children,
                source_path,
                context,
                macro_depth + 1,
                include_loader,
            );
            context.properties = previous;
            output.extend(expanded?);
            continue;
        }

        let mut expanded = node.clone();
        for (name, value) in &mut expanded.attributes {
            if !name.starts_with("xmlns") {
                *value = substitute(value, &context.properties)?;
            }
        }
        expanded.text = substitute(&expanded.text, &context.properties)?;
        expanded.children = expand_nodes(
            &node.children,
            source_path,
            context,
            macro_depth,
            include_loader,
        )?;
        context.expanded_bytes = context
            .expanded_bytes
            .saturating_add(expanded.name.len())
            .saturating_add(expanded.text.len())
            .saturating_add(
                expanded
                    .attributes
                    .iter()
                    .map(|(name, value)| name.len() + value.len())
                    .sum::<usize>(),
            );
        if context.expanded_bytes > MAX_OUTPUT_BYTES {
            return Err(format!(
                "Xacro expansion exceeds the {MAX_OUTPUT_BYTES}-byte output limit"
            ));
        }
        output.push(expanded);
    }
    Ok(output)
}

fn write_node(writer: &mut Writer<Cursor<Vec<u8>>>, node: &XmlNode) -> Result<(), String> {
    let mut start = BytesStart::new(node.name.as_str());
    for (name, value) in &node.attributes {
        start.push_attribute((name.as_str(), value.as_str()));
    }
    if node.children.is_empty() && node.text.is_empty() {
        writer
            .write_event(Event::Empty(start))
            .map_err(|error| format!("Could not serialize expanded Xacro: {error}"))?;
        return Ok(());
    }
    writer
        .write_event(Event::Start(start))
        .map_err(|error| format!("Could not serialize expanded Xacro: {error}"))?;
    if !node.text.is_empty() {
        writer
            .write_event(Event::Text(BytesText::new(&node.text)))
            .map_err(|error| format!("Could not serialize expanded Xacro: {error}"))?;
    }
    for child in &node.children {
        write_node(writer, child)?;
    }
    writer
        .write_event(Event::End(BytesEnd::new(node.name.as_str())))
        .map_err(|error| format!("Could not serialize expanded Xacro: {error}"))?;
    Ok(())
}

pub fn expand<F>(
    content: &str,
    source_path: &Path,
    include_loader: &mut F,
) -> Result<String, String>
where
    F: FnMut(&Path, &str) -> Result<(PathBuf, String), String>,
{
    let deadline = Instant::now() + MAX_EXPANSION_TIME;
    let root = parse_document(content, deadline)?;
    if local_name(&root.name) != "robot" {
        return Err("Xacro document must have a <robot> root".to_string());
    }
    let mut context = Context {
        properties: HashMap::new(),
        macros: HashMap::new(),
        include_stack: HashSet::new(),
        expanded_nodes: 0,
        expanded_bytes: 0,
        include_count: 0,
        deadline,
    };
    if source_path.is_absolute() {
        context.include_stack.insert(source_path.to_path_buf());
    }
    let expanded_children =
        expand_nodes(&root.children, source_path, &mut context, 0, include_loader)?;
    let mut expanded_root = root;
    for (name, value) in &mut expanded_root.attributes {
        if !name.starts_with("xmlns") {
            *value = substitute(value, &context.properties)?;
        }
    }
    expanded_root.text = substitute(&expanded_root.text, &context.properties)?;
    expanded_root.children = expanded_children;
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    write_node(&mut writer, &expanded_root)?;
    let bytes = writer.into_inner().into_inner();
    if bytes.len() > MAX_OUTPUT_BYTES {
        return Err(format!(
            "Expanded Xacro exceeds the {MAX_OUTPUT_BYTES}-byte limit"
        ));
    }
    String::from_utf8(bytes).map_err(|error| format!("Expanded Xacro is not UTF-8: {error}"))
}

#[cfg(test)]
mod tests {
    use super::expand;
    use std::path::Path;

    #[test]
    fn expands_properties_arguments_macros_and_conditionals_without_evaluating_code() {
        let input = r#"<robot name="test" xmlns:xacro="http://www.ros.org/wiki/xacro">
            <xacro:arg name="prefix" default="arm"/>
            <xacro:property name="root" value="${prefix}_base"/>
            <xacro:property name="mesh" value="$(find demo_robot)/meshes/base.stl"/>
            <xacro:macro name="link" params="name color">
                <link name="${name}">
                    <origin rpy="${pi/2} 0 0"/>
                    <visual><geometry><box size="1 1 1"/></geometry>
                    <material name="body"><color rgba="${color}"/></material></visual>
                </link>
            </xacro:macro>
            <xacro:link name="${root}" color="0.2 0.3 0.4 1"/>
            <link name="resource" filename="${mesh}"/>
            <xacro:if value="true"><link name="tool"/></xacro:if>
            <xacro:unless value="false"><link name="sensor"/></xacro:unless>
        </robot>"#;
        let mut include_loader = |_: &Path, _: &str| Err("no includes expected".to_string());
        let expanded = expand(input, Path::new("robot.xacro"), &mut include_loader).unwrap();
        assert!(expanded.contains("name=\"arm_base\""));
        assert!(expanded.contains("name=\"tool\""));
        assert!(expanded.contains("name=\"sensor\""));
        assert!(expanded.contains("filename=\"package://demo_robot/meshes/base.stl\""));
        assert!(!expanded.contains("xacro:"));
        assert!(expanded.contains("rpy=\"1.5707963267948966 0 0\""));
    }

    #[test]
    fn expands_relative_includes_and_rejects_cycles() {
        let input = r#"<robot name="test" xmlns:xacro="http://www.ros.org/wiki/xacro"><xacro:include filename="parts.xacro"/></robot>"#;
        let mut include_loader = |_: &Path, filename: &str| {
            assert_eq!(filename, "parts.xacro");
            Ok((
                Path::new("/workspace/parts.xacro").to_path_buf(),
                r#"<robot name="included"><link name="base"/></robot>"#.to_string(),
            ))
        };
        let expanded = expand(
            input,
            Path::new("/workspace/main.xacro"),
            &mut include_loader,
        )
        .unwrap();
        assert!(expanded.contains("<link name=\"base\"/>"));

        let mut cycle_loader = |_: &Path, _: &str| {
            Ok((
                Path::new("/workspace/main.xacro").to_path_buf(),
                r#"<robot name="test"/>"#.to_string(),
            ))
        };
        assert!(
            expand(input, Path::new("/workspace/main.xacro"), &mut cycle_loader)
                .unwrap_err()
                .contains("cycle")
        );
    }

    #[test]
    fn rejects_expression_execution_and_unbounded_expansion() {
        let mut include_loader = |_: &Path, _: &str| Err("no includes expected".to_string());
        let code = r#"<robot name="test" xmlns:xacro="http://www.ros.org/wiki/xacro"><link name="${__import__('os').system('x')}"/></robot>"#;
        assert!(expand(code, Path::new("robot.xacro"), &mut include_loader)
            .unwrap_err()
            .contains("not defined"));
        let looped = r#"<robot name="test" xmlns:xacro="http://www.ros.org/wiki/xacro"><xacro:macro name="again" params=""><xacro:again/></xacro:macro><xacro:again/></robot>"#;
        assert!(
            expand(looped, Path::new("robot.xacro"), &mut include_loader)
                .unwrap_err()
                .contains("recursion")
        );
    }
}
