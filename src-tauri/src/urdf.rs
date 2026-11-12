use serde::Serialize;
use std::collections::{HashMap, HashSet};

const MAX_DESCRIPTION_BYTES: usize = 4 * 1024 * 1024;
const MAX_MESH_BYTES: usize = 8 * 1024 * 1024;
const MAX_LINKS: usize = 512;
const MAX_JOINTS: usize = 512;
const MAX_VISUALS_PER_LINK: usize = 64;
const MAX_MESH_TRIANGLES: usize = 250_000;
const MAX_XML_ELEMENTS: usize = 100_000;
const MAX_XML_DEPTH: usize = 128;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotOrigin {
    pub xyz: [f64; 3],
    pub rpy: [f64; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RobotGeometry {
    Box {
        size: [f64; 3],
    },
    Cylinder {
        radius: f64,
        length: f64,
    },
    Sphere {
        radius: f64,
    },
    Mesh {
        filename: String,
        scale: [f64; 3],
        format: String,
        data: Option<String>,
    },
    Unsupported {
        description: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotVisual {
    pub origin: RobotOrigin,
    pub geometry: RobotGeometry,
    pub color: Option<[f64; 4]>,
    pub material_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotInertial {
    pub origin: RobotOrigin,
    pub mass: f64,
    pub inertia: [f64; 6],
}

#[derive(Default)]
struct InertialBuilder {
    origin: RobotOrigin,
    mass: Option<f64>,
    inertia: Option<[f64; 6]>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotLink {
    pub name: String,
    pub visuals: Vec<RobotVisual>,
    pub collisions: Vec<RobotVisual>,
    pub inertial: Option<RobotInertial>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotJointLimit {
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub effort: Option<f64>,
    pub velocity: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotMimic {
    pub joint: String,
    pub multiplier: f64,
    pub offset: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotJoint {
    pub name: String,
    pub joint_type: String,
    pub parent: String,
    pub child: String,
    pub origin: RobotOrigin,
    pub axis: [f64; 3],
    pub limit: Option<RobotJointLimit>,
    pub mimic: Option<RobotMimic>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotDescription {
    pub name: String,
    pub root_link: String,
    pub links: Vec<RobotLink>,
    pub joints: Vec<RobotJoint>,
    pub warnings: Vec<String>,
}

fn attribute_map(
    element: &quick_xml::events::BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
) -> Result<HashMap<String, String>, String> {
    let mut result = HashMap::new();
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| format!("Malformed URDF attribute: {error}"))?;
        let name = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
        let value = attribute
            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, decoder)
            .map_err(|error| format!("Malformed URDF attribute value: {error}"))?
            .into_owned();
        result.insert(name, value);
    }
    Ok(result)
}

fn required_attribute<'a>(
    attributes: &'a HashMap<String, String>,
    name: &str,
) -> Result<&'a str, String> {
    attributes
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("URDF element is missing its '{name}' attribute"))
}

fn parse_number(value: &str, description: &str) -> Result<f64, String> {
    let number = value
        .parse::<f64>()
        .map_err(|_| format!("Invalid {description}: expected a number"))?;
    if !number.is_finite() {
        return Err(format!("Invalid {description}: value must be finite"));
    }
    Ok(number)
}

fn parse_vector<const N: usize>(value: &str, description: &str) -> Result<[f64; N], String> {
    let values = value
        .split_whitespace()
        .map(|part| parse_number(part, description))
        .collect::<Result<Vec<_>, _>>()?;
    values
        .try_into()
        .map_err(|_| format!("Invalid {description}: expected {N} values"))
}

fn parse_origin(attributes: &HashMap<String, String>) -> Result<RobotOrigin, String> {
    Ok(RobotOrigin {
        xyz: attributes
            .get("xyz")
            .map(|value| parse_vector(value, "origin xyz"))
            .transpose()?
            .unwrap_or([0.0; 3]),
        rpy: attributes
            .get("rpy")
            .map(|value| parse_vector(value, "origin rpy"))
            .transpose()?
            .unwrap_or([0.0; 3]),
    })
}

fn parse_optional_number(
    attributes: &HashMap<String, String>,
    name: &str,
    description: &str,
) -> Result<Option<f64>, String> {
    attributes
        .get(name)
        .map(|value| parse_number(value, description))
        .transpose()
}

fn parse_joint_limit(attributes: &HashMap<String, String>) -> Result<RobotJointLimit, String> {
    let limit = RobotJointLimit {
        lower: parse_optional_number(attributes, "lower", "joint lower limit")?,
        upper: parse_optional_number(attributes, "upper", "joint upper limit")?,
        effort: parse_optional_number(attributes, "effort", "joint effort limit")?,
        velocity: parse_optional_number(attributes, "velocity", "joint velocity limit")?,
    };
    if limit
        .lower
        .zip(limit.upper)
        .is_some_and(|(lower, upper)| lower > upper)
    {
        return Err("URDF joint lower limit must not exceed its upper limit".to_string());
    }
    if limit.effort.is_some_and(|value| value <= 0.0)
        || limit.velocity.is_some_and(|value| value <= 0.0)
    {
        return Err("URDF joint effort and velocity limits must be positive".to_string());
    }
    Ok(limit)
}

fn parse_mimic(attributes: &HashMap<String, String>) -> Result<RobotMimic, String> {
    Ok(RobotMimic {
        joint: required_attribute(attributes, "joint")?.to_string(),
        multiplier: parse_optional_number(attributes, "multiplier", "mimic multiplier")?
            .unwrap_or(1.0),
        offset: parse_optional_number(attributes, "offset", "mimic offset")?.unwrap_or(0.0),
    })
}

fn parse_color(value: &str) -> Result<[f64; 4], String> {
    let color = parse_vector(value, "material rgba")?;
    if color
        .iter()
        .any(|component| !(0.0..=1.0).contains(component))
    {
        return Err("Invalid material rgba: components must be between 0 and 1".to_string());
    }
    Ok(color)
}

pub fn validate_mesh_bytes(format: &str, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_MESH_BYTES {
        return Err(format!(
            "Mesh size must be between 1 byte and {MAX_MESH_BYTES} bytes"
        ));
    }

    match format {
        "stl" => {
            let triangle_count = if bytes.len() >= 84 {
                let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
                let expected_size = 84usize.saturating_add(count.saturating_mul(50));
                (expected_size == bytes.len()).then_some(count)
            } else {
                None
            };
            let triangle_count = match triangle_count {
                Some(count) => count,
                None => {
                    let text = std::str::from_utf8(bytes).map_err(|_| {
                        "STL mesh is neither valid binary nor UTF-8 ASCII".to_string()
                    })?;
                    if !text.trim_start().starts_with("solid")
                        || !text
                            .trim_end()
                            .lines()
                            .last()
                            .is_some_and(|line| line.trim_start().starts_with("endsolid"))
                    {
                        return Err("Malformed ASCII STL mesh".to_string());
                    }
                    text.match_indices("facet normal").count()
                }
            };
            if triangle_count == 0 || triangle_count > MAX_MESH_TRIANGLES {
                return Err(format!(
                    "STL triangle count must be between 1 and {MAX_MESH_TRIANGLES}"
                ));
            }
        }
        "dae" => {
            let text =
                std::str::from_utf8(bytes).map_err(|_| "DAE mesh must be UTF-8 XML".to_string())?;
            if text.len() > MAX_MESH_BYTES {
                return Err("DAE mesh exceeds its byte limit".to_string());
            }
            let mut reader = quick_xml::Reader::from_str(text);
            reader.config_mut().trim_text(true);
            let mut root_seen = false;
            let mut triangle_count = 0usize;
            let mut node_count = 0usize;
            let mut depth = 0usize;
            loop {
                match reader.read_event() {
                    Ok(
                        event @ (quick_xml::events::Event::Start(_)
                        | quick_xml::events::Event::Empty(_)),
                    ) => {
                        let (element, is_empty) = match event {
                            quick_xml::events::Event::Start(element) => (element, false),
                            quick_xml::events::Event::Empty(element) => (element, true),
                            _ => unreachable!(),
                        };
                        node_count += 1;
                        if node_count > 100_000 || depth > 128 {
                            return Err("DAE XML exceeds the node or nesting limit".to_string());
                        }
                        let name =
                            String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                        if !root_seen {
                            if name != "COLLADA" {
                                return Err("DAE mesh root element must be <COLLADA>".to_string());
                            }
                            root_seen = true;
                        }
                        if name == "image" {
                            return Err(
                                "DAE meshes with external image/texture references are not supported"
                                    .to_string(),
                            );
                        }
                        for attribute in element.attributes().with_checks(true) {
                            let attribute = attribute
                                .map_err(|error| format!("Malformed DAE attribute: {error}"))?;
                            if String::from_utf8_lossy(attribute.key.as_ref())
                                .rsplit(':')
                                .next()
                                == Some("url")
                            {
                                let value = attribute
                                    .decoded_and_normalized_value(
                                        quick_xml::XmlVersion::Implicit1_0,
                                        reader.decoder(),
                                    )
                                    .map_err(|error| {
                                        format!("Malformed DAE URL attribute: {error}")
                                    })?;
                                if !value.starts_with('#') || value.len() < 2 {
                                    return Err("DAE meshes cannot reference external resources"
                                        .to_string());
                                }
                            }
                        }
                        if matches!(name.as_ref(), "triangles" | "polylist") {
                            let mut count = None;
                            for attribute in element.attributes().with_checks(true) {
                                let attribute = attribute
                                    .map_err(|error| format!("Malformed DAE attribute: {error}"))?;
                                if attribute.key.as_ref() == b"count" {
                                    let value = attribute
                                        .decoded_and_normalized_value(
                                            quick_xml::XmlVersion::Implicit1_0,
                                            reader.decoder(),
                                        )
                                        .map_err(|error| {
                                            format!("Malformed DAE count attribute: {error}")
                                        })?;
                                    count =
                                        Some(value.parse::<usize>().map_err(|_| {
                                            "Invalid DAE primitive count".to_string()
                                        })?);
                                }
                            }
                            triangle_count =
                                triangle_count.saturating_add(count.ok_or_else(|| {
                                    "DAE primitive is missing its count attribute".to_string()
                                })?);
                            if triangle_count > MAX_MESH_TRIANGLES {
                                return Err(format!(
                                    "DAE triangle count exceeds {MAX_MESH_TRIANGLES}"
                                ));
                            }
                        }
                        if !is_empty {
                            depth += 1;
                        }
                    }
                    Ok(quick_xml::events::Event::End(_)) => {
                        depth = depth.saturating_sub(1);
                    }
                    Ok(quick_xml::events::Event::DocType(_)) => {
                        return Err("DAE documents must not contain a DTD".to_string());
                    }
                    Ok(quick_xml::events::Event::Eof) => break,
                    Ok(_) => {}
                    Err(error) => return Err(format!("Malformed DAE XML: {error}")),
                }
            }
            if !root_seen || triangle_count == 0 {
                return Err("DAE mesh contains no supported triangle geometry".to_string());
            }
        }
        _ => return Err(format!("Unsupported mesh format '{format}'")),
    }
    Ok(())
}

fn element_name(element: &quick_xml::events::BytesStart<'_>) -> String {
    String::from_utf8_lossy(element.local_name().as_ref()).into_owned()
}

fn parse_geometry(
    element_name: &str,
    attributes: &HashMap<String, String>,
) -> Result<RobotGeometry, String> {
    match element_name {
        "box" => {
            let size = parse_vector(required_attribute(attributes, "size")?, "box size")?;
            if size.iter().any(|dimension| *dimension <= 0.0) {
                return Err("URDF box dimensions must be positive".to_string());
            }
            Ok(RobotGeometry::Box { size })
        }
        "cylinder" => {
            let radius =
                parse_number(required_attribute(attributes, "radius")?, "cylinder radius")?;
            let length =
                parse_number(required_attribute(attributes, "length")?, "cylinder length")?;
            if radius <= 0.0 || length <= 0.0 {
                return Err("URDF cylinder dimensions must be positive".to_string());
            }
            Ok(RobotGeometry::Cylinder { radius, length })
        }
        "sphere" => {
            let radius = parse_number(required_attribute(attributes, "radius")?, "sphere radius")?;
            if radius <= 0.0 {
                return Err("URDF sphere radius must be positive".to_string());
            }
            Ok(RobotGeometry::Sphere { radius })
        }
        "mesh" => {
            let filename = required_attribute(attributes, "filename")?.to_string();
            let format = filename
                .rsplit_once('.')
                .map(|(_, extension)| extension.to_ascii_lowercase())
                .ok_or_else(|| format!("Unsupported URDF mesh format: '{filename}'"))?;
            if !matches!(format.as_str(), "stl" | "dae") {
                return Err(format!(
                    "Unsupported URDF mesh format '{format}'; only STL and DAE are supported"
                ));
            }
            let scale = attributes
                .get("scale")
                .map(|value| parse_vector(value, "mesh scale"))
                .transpose()?
                .unwrap_or([1.0; 3]);
            if scale
                .iter()
                .any(|component| component.abs() <= f64::EPSILON)
            {
                return Err("URDF mesh scale components must be non-zero".to_string());
            }
            Ok(RobotGeometry::Mesh {
                filename,
                scale,
                format,
                data: None,
            })
        }
        _ => Err(format!("Unsupported URDF geometry element: {element_name}")),
    }
}

fn validate_robot_tree(links: &[RobotLink], joints: &[RobotJoint]) -> Result<String, String> {
    if links.is_empty() {
        return Err("URDF must define at least one link".to_string());
    }
    let link_names = links
        .iter()
        .map(|link| link.name.as_str())
        .collect::<HashSet<_>>();
    if link_names.len() != links.len() {
        return Err("URDF contains duplicate link names".to_string());
    }
    let joint_names = joints
        .iter()
        .map(|joint| joint.name.as_str())
        .collect::<HashSet<_>>();
    if joint_names.len() != joints.len() {
        return Err("URDF contains duplicate joint names".to_string());
    }
    let joints_by_name = joints
        .iter()
        .map(|joint| (joint.name.as_str(), joint))
        .collect::<HashMap<_, _>>();
    for joint in joints {
        if let Some(mimic) = &joint.mimic {
            if !joints_by_name.contains_key(mimic.joint.as_str()) {
                return Err(format!(
                    "Joint '{}' mimics undefined joint '{}'",
                    joint.name, mimic.joint
                ));
            }
            if mimic.joint == joint.name {
                return Err(format!("Joint '{}' cannot mimic itself", joint.name));
            }
        }
    }

    fn visit_mimic(
        name: &str,
        joints: &HashMap<&str, &RobotJoint>,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) -> Result<(), String> {
        if visited.contains(name) {
            return Ok(());
        }
        if !visiting.insert(name.to_string()) {
            return Err("URDF joint mimic relationships contain a cycle".to_string());
        }
        if let Some(source) = joints.get(name).and_then(|joint| joint.mimic.as_ref()) {
            visit_mimic(&source.joint, joints, visiting, visited)?;
        }
        visiting.remove(name);
        visited.insert(name.to_string());
        Ok(())
    }
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for joint in joints {
        visit_mimic(&joint.name, &joints_by_name, &mut visiting, &mut visited)?;
    }

    let mut child_links = HashSet::new();
    let mut children_by_parent: HashMap<&str, Vec<&str>> = HashMap::new();
    for joint in joints {
        if !link_names.contains(joint.parent.as_str()) || !link_names.contains(joint.child.as_str())
        {
            return Err(format!(
                "Joint '{}' refers to an undefined parent or child link",
                joint.name
            ));
        }
        if joint.parent == joint.child {
            return Err(format!("Joint '{}' connects a link to itself", joint.name));
        }
        if !child_links.insert(joint.child.as_str()) {
            return Err(format!(
                "Link '{}' has more than one parent joint",
                joint.child
            ));
        }
        children_by_parent
            .entry(&joint.parent)
            .or_default()
            .push(&joint.child);
    }

    let roots = links
        .iter()
        .filter(|link| !child_links.contains(link.name.as_str()))
        .map(|link| link.name.as_str())
        .collect::<Vec<_>>();
    if roots.len() != 1 {
        return Err("URDF links must form one connected tree with one root".to_string());
    }

    let mut visited = HashSet::new();
    let mut pending = vec![roots[0]];
    while let Some(link) = pending.pop() {
        if !visited.insert(link) {
            return Err("URDF joint graph contains a cycle".to_string());
        }
        if let Some(children) = children_by_parent.get(link) {
            pending.extend(children.iter().copied());
        }
    }
    if visited.len() != links.len() {
        return Err("URDF joint graph is disconnected or cyclic".to_string());
    }

    Ok(roots[0].to_string())
}

pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String> {
    if content.len() > MAX_DESCRIPTION_BYTES {
        return Err(format!(
            "URDF exceeds the {MAX_DESCRIPTION_BYTES}-byte limit"
        ));
    }
    if content.contains('\0') {
        return Err("URDF contains a NUL character".to_string());
    }

    let mut reader = quick_xml::Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut stack = Vec::<String>::new();
    let mut robot_name = None;
    let mut root_seen = false;
    let mut root_closed = false;
    let mut current_link = None::<RobotLink>;
    let mut current_joint = None::<RobotJoint>;
    let mut current_visual = None::<RobotVisual>;
    let mut current_collision = None::<RobotVisual>;
    let mut current_inertial = None::<InertialBuilder>;
    let mut current_global_material = None::<(String, Option<[f64; 4]>)>;
    let mut global_materials = HashMap::<String, Option<[f64; 4]>>::new();
    let mut links = Vec::new();
    let mut joints = Vec::new();
    let mut warnings = Vec::new();
    let mut element_count = 0usize;

    loop {
        match reader.read_event() {
            Ok(
                event @ (quick_xml::events::Event::Start(_) | quick_xml::events::Event::Empty(_)),
            ) => {
                element_count += 1;
                if element_count > MAX_XML_ELEMENTS || stack.len() >= MAX_XML_DEPTH {
                    return Err("URDF XML exceeds the element or nesting limit".to_string());
                }
                let (element, is_empty) = match event {
                    quick_xml::events::Event::Start(element) => (element, false),
                    quick_xml::events::Event::Empty(element) => (element, true),
                    _ => unreachable!(),
                };
                let name = element_name(&element);
                let parent = stack.last().map(String::as_str);
                if root_closed {
                    return Err("URDF contains content after its root element".to_string());
                }
                let attributes = attribute_map(&element, reader.decoder())?;

                match name.as_str() {
                    "robot" if stack.is_empty() && !root_seen => {
                        let name = required_attribute(&attributes, "name")?;
                        if name.len() > 128 {
                            return Err("URDF robot name exceeds 128 bytes".to_string());
                        }
                        robot_name = Some(name.to_string());
                        root_seen = true;
                    }
                    "robot" => return Err("URDF must contain exactly one <robot> root".to_string()),
                    "link" if parent == Some("robot") => {
                        if current_link.is_some() {
                            return Err("Nested URDF <link> elements are invalid".to_string());
                        }
                        current_link = Some(RobotLink {
                            name: required_attribute(&attributes, "name")?.to_string(),
                            visuals: Vec::new(),
                            collisions: Vec::new(),
                            inertial: None,
                        });
                    }
                    "joint" if parent == Some("robot") => {
                        if current_joint.is_some() {
                            return Err("Nested URDF <joint> elements are invalid".to_string());
                        }
                        current_joint = Some(RobotJoint {
                            name: required_attribute(&attributes, "name")?.to_string(),
                            joint_type: required_attribute(&attributes, "type")?.to_string(),
                            parent: String::new(),
                            child: String::new(),
                            origin: RobotOrigin::default(),
                            axis: [1.0, 0.0, 0.0],
                            limit: None,
                            mimic: None,
                        });
                    }
                    "visual" if parent == Some("link") => {
                        if current_visual.is_some() {
                            return Err("Nested URDF <visual> elements are invalid".to_string());
                        }
                        current_visual = Some(RobotVisual {
                            origin: RobotOrigin::default(),
                            geometry: RobotGeometry::Unsupported {
                                description: String::new(),
                            },
                            color: None,
                            material_name: None,
                        });
                    }
                    "collision" if parent == Some("link") => {
                        if current_collision.is_some() {
                            return Err("Nested URDF <collision> elements are invalid".to_string());
                        }
                        current_collision = Some(RobotVisual {
                            origin: RobotOrigin::default(),
                            geometry: RobotGeometry::Unsupported {
                                description: String::new(),
                            },
                            color: None,
                            material_name: None,
                        });
                    }
                    "origin" if parent == Some("visual") => {
                        if let Some(visual) = current_visual.as_mut() {
                            visual.origin = parse_origin(&attributes)?;
                        }
                    }
                    "origin" if parent == Some("collision") => {
                        if let Some(collision) = current_collision.as_mut() {
                            collision.origin = parse_origin(&attributes)?;
                        }
                    }
                    "origin" if parent == Some("joint") => {
                        if let Some(joint) = current_joint.as_mut() {
                            joint.origin = parse_origin(&attributes)?;
                        }
                    }
                    "origin" if parent == Some("inertial") => {
                        if let Some(inertial) = current_inertial.as_mut() {
                            inertial.origin = parse_origin(&attributes)?;
                        }
                    }
                    "parent" if parent == Some("joint") => {
                        if let Some(joint) = current_joint.as_mut() {
                            joint.parent = required_attribute(&attributes, "link")?.to_string();
                        }
                    }
                    "child" if parent == Some("joint") => {
                        if let Some(joint) = current_joint.as_mut() {
                            joint.child = required_attribute(&attributes, "link")?.to_string();
                        }
                    }
                    "axis" if parent == Some("joint") => {
                        if let Some(joint) = current_joint.as_mut() {
                            if let Some(axis) = attributes.get("xyz") {
                                joint.axis = parse_vector(axis, "joint axis")?;
                            }
                        }
                    }
                    "limit" if parent == Some("joint") => {
                        if let Some(joint) = current_joint.as_mut() {
                            if joint.limit.is_some() {
                                return Err(format!(
                                    "Joint '{}' defines more than one limit",
                                    joint.name
                                ));
                            }
                            joint.limit = Some(parse_joint_limit(&attributes)?);
                        }
                    }
                    "mimic" if parent == Some("joint") => {
                        if let Some(joint) = current_joint.as_mut() {
                            if joint.mimic.is_some() {
                                return Err(format!(
                                    "Joint '{}' defines more than one mimic relationship",
                                    joint.name
                                ));
                            }
                            joint.mimic = Some(parse_mimic(&attributes)?);
                        }
                    }
                    "inertial" if parent == Some("link") => {
                        if current_inertial.is_some() {
                            return Err("Link contains more than one inertial element".to_string());
                        }
                        current_inertial = Some(InertialBuilder::default());
                    }
                    "mass" if parent == Some("inertial") => {
                        if let Some(inertial) = current_inertial.as_mut() {
                            if inertial.mass.is_some() {
                                return Err("Inertial element defines mass more than once".into());
                            }
                            let mass = parse_number(
                                required_attribute(&attributes, "value")?,
                                "inertial mass",
                            )?;
                            if mass < 0.0 {
                                return Err("URDF inertial mass must not be negative".to_string());
                            }
                            inertial.mass = Some(mass);
                        }
                    }
                    "inertia" if parent == Some("inertial") => {
                        if let Some(inertial) = current_inertial.as_mut() {
                            if inertial.inertia.is_some() {
                                return Err(
                                    "Inertial element defines inertia more than once".into()
                                );
                            }
                            let names = ["ixx", "ixy", "ixz", "iyy", "iyz", "izz"];
                            let mut values = [0.0; 6];
                            for (index, name) in names.iter().enumerate() {
                                values[index] = parse_number(
                                    required_attribute(&attributes, name)?,
                                    "inertia tensor",
                                )?;
                            }
                            let [ixx, ixy, ixz, iyy, iyz, izz] = values;
                            let minor_xy = ixx * iyy - ixy * ixy;
                            let minor_xz = ixx * izz - ixz * ixz;
                            let minor_yz = iyy * izz - iyz * iyz;
                            let determinant = ixx * (iyy * izz - iyz * iyz)
                                - ixy * (ixy * izz - iyz * ixz)
                                + ixz * (ixy * iyz - iyy * ixz);
                            if !minor_xy.is_finite()
                                || !minor_xz.is_finite()
                                || !minor_yz.is_finite()
                                || !determinant.is_finite()
                                || ixx < 0.0
                                || iyy < 0.0
                                || izz < 0.0
                                || minor_xy < -f64::EPSILON
                                || minor_xz < -f64::EPSILON
                                || minor_yz < -f64::EPSILON
                                || determinant < -f64::EPSILON
                            {
                                return Err(
                                    "URDF inertia tensor must be positive semidefinite".to_string()
                                );
                            }
                            inertial.inertia = Some(values);
                        }
                    }
                    "material" if parent == Some("robot") => {
                        let name = required_attribute(&attributes, "name")?.to_string();
                        if current_global_material.is_some() {
                            return Err("Nested URDF material elements are invalid".into());
                        }
                        current_global_material = Some((name, None));
                    }
                    "material" if parent == Some("visual") => {
                        if let Some(visual) = current_visual.as_mut() {
                            visual.material_name = attributes.get("name").cloned();
                        }
                    }
                    "color" if parent == Some("material") => {
                        if let Some(color) = attributes.get("rgba") {
                            let color = parse_color(color)?;
                            if let Some((_, material_color)) = current_global_material.as_mut() {
                                *material_color = Some(color);
                            } else if let Some(visual) = current_visual.as_mut() {
                                visual.color = Some(color);
                            }
                        }
                    }
                    "box" | "cylinder" | "sphere" | "mesh" if parent == Some("geometry") => {
                        if let Some(visual) = current_visual.as_mut().or(current_collision.as_mut())
                        {
                            if !matches!(
                                visual.geometry,
                                RobotGeometry::Unsupported { ref description }
                                    if description.is_empty()
                            ) {
                                return Err(
                                    "URDF visual must contain exactly one geometry".to_string()
                                );
                            }
                            visual.geometry = parse_geometry(&name, &attributes)?;
                        }
                    }
                    _ => {}
                }

                if is_empty {
                    match name.as_str() {
                        "robot" => root_closed = true,
                        "link" => {
                            if links.len() >= MAX_LINKS {
                                return Err(format!("URDF contains more than {MAX_LINKS} links"));
                            }
                            links.push(
                                current_link
                                    .take()
                                    .ok_or_else(|| "URDF link state is inconsistent".to_string())?,
                            );
                        }
                        "inertial" => {
                            let inertial = current_inertial
                                .take()
                                .ok_or_else(|| "URDF inertial state is inconsistent".to_string())?;
                            let mass = inertial
                                .mass
                                .ok_or_else(|| "URDF inertial is missing mass".to_string())?;
                            let inertia = inertial.inertia.ok_or_else(|| {
                                "URDF inertial is missing its inertia tensor".to_string()
                            })?;
                            if let Some(link) = current_link.as_mut() {
                                link.inertial = Some(RobotInertial {
                                    origin: inertial.origin,
                                    mass,
                                    inertia,
                                });
                            } else {
                                return Err("URDF inertial is outside a link".to_string());
                            }
                        }
                        "material" if current_global_material.is_some() => {
                            let (name, color) = current_global_material
                                .take()
                                .ok_or_else(|| "URDF material state is inconsistent".to_string())?;
                            if global_materials.insert(name.clone(), color).is_some() {
                                return Err(format!(
                                    "URDF material '{name}' is defined more than once"
                                ));
                            }
                        }
                        "visual" => {
                            return Err("URDF visual is missing a geometry".to_string());
                        }
                        "collision" => {
                            return Err("URDF collision is missing a geometry".to_string());
                        }
                        "joint" => {
                            return Err("URDF joint must define parent and child links".to_string());
                        }
                        _ => {}
                    }
                } else {
                    stack.push(name);
                }
            }
            Ok(quick_xml::events::Event::End(element)) => {
                let name = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                if stack.pop().as_deref() != Some(name.as_str()) {
                    return Err("URDF XML elements are improperly nested".to_string());
                }
                match name.as_str() {
                    "visual" => {
                        let visual = current_visual
                            .take()
                            .ok_or_else(|| "URDF visual state is inconsistent".to_string())?;
                        if matches!(
                            visual.geometry,
                            RobotGeometry::Unsupported { ref description }
                                if description.is_empty()
                        ) {
                            return Err("URDF visual is missing a supported geometry".to_string());
                        }
                        if let RobotGeometry::Unsupported { description } = &visual.geometry {
                            warnings.push(format!(
                                "Mesh geometry '{description}' is not rendered in this preview"
                            ));
                        }
                        let link = current_link
                            .as_mut()
                            .ok_or_else(|| "URDF visual is outside a link".to_string())?;
                        if link.visuals.len() >= MAX_VISUALS_PER_LINK {
                            return Err(format!(
                                "Link '{}' contains too many visual elements",
                                link.name
                            ));
                        }
                        link.visuals.push(visual);
                    }
                    "collision" => {
                        let collision = current_collision
                            .take()
                            .ok_or_else(|| "URDF collision state is inconsistent".to_string())?;
                        if matches!(
                            collision.geometry,
                            RobotGeometry::Unsupported { ref description }
                                if description.is_empty()
                        ) {
                            return Err(
                                "URDF collision is missing a supported geometry".to_string()
                            );
                        }
                        if let RobotGeometry::Unsupported { description } = &collision.geometry {
                            warnings.push(format!(
                                "Collision mesh geometry '{description}' is not rendered in this preview"
                            ));
                        }
                        let link = current_link
                            .as_mut()
                            .ok_or_else(|| "URDF collision is outside a link".to_string())?;
                        if link.collisions.len() >= MAX_VISUALS_PER_LINK {
                            return Err(format!(
                                "Link '{}' contains too many collision elements",
                                link.name
                            ));
                        }
                        link.collisions.push(collision);
                    }
                    "inertial" => {
                        let inertial = current_inertial
                            .take()
                            .ok_or_else(|| "URDF inertial state is inconsistent".to_string())?;
                        let mass = inertial
                            .mass
                            .ok_or_else(|| "URDF inertial is missing mass".to_string())?;
                        let inertia = inertial.inertia.ok_or_else(|| {
                            "URDF inertial is missing its inertia tensor".to_string()
                        })?;
                        let link = current_link
                            .as_mut()
                            .ok_or_else(|| "URDF inertial is outside a link".to_string())?;
                        link.inertial = Some(RobotInertial {
                            origin: inertial.origin,
                            mass,
                            inertia,
                        });
                    }
                    "material" if current_global_material.is_some() => {
                        let (name, color) = current_global_material
                            .take()
                            .ok_or_else(|| "URDF material state is inconsistent".to_string())?;
                        if global_materials.insert(name.clone(), color).is_some() {
                            return Err(format!(
                                "URDF material '{name}' is defined more than once"
                            ));
                        }
                    }
                    "link" => {
                        if links.len() >= MAX_LINKS {
                            return Err(format!("URDF contains more than {MAX_LINKS} links"));
                        }
                        links.push(
                            current_link
                                .take()
                                .ok_or_else(|| "URDF link state is inconsistent".to_string())?,
                        );
                    }
                    "joint" => {
                        if joints.len() >= MAX_JOINTS {
                            return Err(format!("URDF contains more than {MAX_JOINTS} joints"));
                        }
                        let mut joint = current_joint
                            .take()
                            .ok_or_else(|| "URDF joint state is inconsistent".to_string())?;
                        if joint.parent.is_empty() || joint.child.is_empty() {
                            return Err(format!(
                                "Joint '{}' must define parent and child links",
                                joint.name
                            ));
                        }
                        match joint.joint_type.as_str() {
                            "fixed" => {}
                            "revolute" | "continuous" | "prismatic" => {
                                let magnitude =
                                    joint.axis[0].hypot(joint.axis[1]).hypot(joint.axis[2]);
                                if magnitude <= f64::EPSILON {
                                    return Err(format!(
                                        "Joint '{}' axis must be non-zero",
                                        joint.name
                                    ));
                                }
                                joint.axis.iter_mut().for_each(|axis| *axis /= magnitude);
                            }
                            "floating" | "planar" => warnings.push(format!(
                                "Joint '{}' uses '{}' motion; the scalar joint-state preview applies a best-effort axis translation only",
                                joint.name, joint.joint_type
                            )),
                            _ => {
                                return Err(format!(
                                    "Joint '{}' has unsupported type '{}'",
                                    joint.name, joint.joint_type
                                ));
                            }
                        }
                        joints.push(joint);
                    }
                    "robot" => root_closed = true,
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::DocType(_)) => {
                return Err("URDF documents must not contain a DTD".to_string());
            }
            Ok(quick_xml::events::Event::Text(text)) if stack.is_empty() => {
                let text_bytes: &[u8] = text.as_ref();
                if !text_bytes.iter().all(u8::is_ascii_whitespace) {
                    return Err("URDF contains text outside its root element".to_string());
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("Malformed URDF XML: {error}")),
        }
    }

    if !root_seen || !root_closed || !stack.is_empty() {
        return Err("URDF document is incomplete".to_string());
    }
    for link in &mut links {
        for visual in &mut link.visuals {
            if visual.color.is_none() {
                if let Some(name) = visual.material_name.as_ref() {
                    match global_materials.get(name) {
                        Some(Some(color)) => visual.color = Some(*color),
                        Some(None) => {}
                        None => {
                            warnings.push(format!("Visual references undefined material '{name}'"))
                        }
                    }
                }
            }
        }
    }
    let root_link = validate_robot_tree(&links, &joints)?;
    let name = robot_name.ok_or_else(|| "URDF robot name is missing".to_string())?;
    Ok(RobotDescription {
        name,
        root_link,
        links,
        joints,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_robot_description, validate_mesh_bytes, RobotGeometry};

    #[test]
    fn parses_primitive_visuals_and_normalizes_joint_axes() {
        let robot = parse_robot_description(
            r#"<robot name="test_bot">
                <link name="base">
                    <visual>
                        <origin xyz="1 2 3" rpy="0 0 1.5"/>
                        <geometry><box size="1 2 3"/></geometry>
                        <material name="blue"><color rgba="0.1 0.2 0.8 1"/></material>
                    </visual>
                </link>
                <link name="wheel"/>
                <joint name="base_to_wheel" type="continuous">
                    <parent link="base"/><child link="wheel"/>
                    <axis xyz="0 0 2"/>
                </joint>
            </robot>"#,
        )
        .unwrap();

        assert_eq!(robot.name, "test_bot");
        assert_eq!(robot.root_link, "base");
        assert_eq!(robot.links[0].visuals[0].origin.xyz, [1.0, 2.0, 3.0]);
        assert_eq!(
            robot.links[0].visuals[0].geometry,
            RobotGeometry::Box {
                size: [1.0, 2.0, 3.0]
            }
        );
        assert_eq!(robot.links[0].visuals[0].color, Some([0.1, 0.2, 0.8, 1.0]));
        assert_eq!(robot.joints[0].axis, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn rejects_dtds_malformed_xml_and_invalid_geometry_dimensions() {
        assert!(parse_robot_description(
            "<!DOCTYPE robot><robot name=\"test\"><link name=\"base\"/></robot>"
        )
        .is_err());
        assert!(
            parse_robot_description("<robot name=\"test\"><link name=\"base\"></robot>").is_err()
        );
        assert!(parse_robot_description(
            r#"<robot name="test"><link name="base"><visual><geometry><sphere radius="0"/></geometry></visual></link></robot>"#
        )
        .is_err());
    }

    #[test]
    fn rejects_disconnected_links_cycles_and_multiple_parent_joints() {
        assert!(parse_robot_description(
            r#"<robot name="test"><link name="a"/><link name="b"/></robot>"#
        )
        .is_err());
        assert!(parse_robot_description(
            r#"<robot name="test"><link name="a"/><link name="b"/><joint name="ab" type="fixed"><parent link="a"/><child link="b"/></joint><joint name="ba" type="fixed"><parent link="b"/><child link="a"/></joint></robot>"#
        )
        .is_err());
        assert!(parse_robot_description(
            r#"<robot name="test"><link name="a"/><link name="b"/><link name="c"/><joint name="ab" type="fixed"><parent link="a"/><child link="b"/></joint><joint name="ac" type="fixed"><parent link="a"/><child link="b"/></joint></robot>"#
        )
        .is_err());
    }

    #[test]
    fn parses_stl_mesh_references_and_scale_for_safe_resource_resolution() {
        let robot = parse_robot_description(
            r#"<robot name="test"><link name="base"><visual><geometry><mesh filename="package://robot/meshes/base.stl" scale="2 1 0.5"/></geometry></visual></link></robot>"#,
        )
        .unwrap();

        let RobotGeometry::Mesh {
            filename,
            scale,
            format,
            data,
        } = &robot.links[0].visuals[0].geometry
        else {
            panic!("expected STL mesh geometry");
        };
        assert_eq!(filename, "package://robot/meshes/base.stl");
        assert_eq!(*scale, [2.0, 1.0, 0.5]);
        assert_eq!(format, "stl");
        assert!(data.is_none());
    }

    #[test]
    fn resolves_named_materials_and_preserves_inertial_frames_and_tensors() {
        let robot = parse_robot_description(
            r#"<robot name="test">
                <material name="blue"><color rgba="0.1 0.2 0.8 1"/></material>
                <link name="base">
                    <inertial><origin xyz="0 0 0.2" rpy="0.1 0.2 0.3"/>
                        <mass value="2"/><inertia ixx="1" ixy="0" ixz="0" iyy="2" iyz="0" izz="3"/>
                    </inertial>
                    <visual><geometry><box size="1 1 1"/></geometry><material name="blue"/></visual>
                </link>
            </robot>"#,
        )
        .unwrap();
        assert_eq!(robot.links[0].visuals[0].color, Some([0.1, 0.2, 0.8, 1.0]));
        let inertial = robot.links[0].inertial.as_ref().unwrap();
        assert_eq!(inertial.mass, 2.0);
        assert_eq!(inertial.origin.xyz, [0.0, 0.0, 0.2]);
        assert_eq!(inertial.inertia, [1.0, 0.0, 0.0, 2.0, 0.0, 3.0]);
    }

    #[test]
    fn rejects_invalid_inertia_and_external_dae_textures_and_bounds_meshes() {
        let invalid_inertia = r#"<robot name="test"><link name="base"><inertial><mass value="1"/><inertia ixx="1" ixy="2" ixz="0" iyy="1" iyz="0" izz="1"/></inertial></link></robot>"#;
        assert!(parse_robot_description(invalid_inertia)
            .unwrap_err()
            .contains("positive semidefinite"));
        let dae = br#"<COLLADA><library_images><image id="texture"><init_from>https://example.test/texture.png</init_from></image></library_images><library_geometries><triangles count="1"/></library_geometries></COLLADA>"#;
        assert!(validate_mesh_bytes("dae", dae)
            .unwrap_err()
            .contains("external image"));
        let external_url =
            br#"<COLLADA><instance_geometry url="https://example.test/geometry.dae"/><triangles count="1"/></COLLADA>"#;
        assert!(validate_mesh_bytes("dae", external_url)
            .unwrap_err()
            .contains("external resources"));
        assert!(validate_mesh_bytes("stl", b"invalid").is_err());
        assert!(validate_mesh_bytes("stl", &vec![0; 8 * 1024 * 1024 + 1]).is_err());
    }

    #[test]
    fn parses_joint_limits_mimic_relationships_and_collision_geometry() {
        let robot = parse_robot_description(
            r#"<robot name="bounded">
                <link name="base"/>
                <link name="follower">
                    <collision>
                        <origin xyz="0 0 0.1"/>
                        <geometry><box size="0.2 0.3 0.4"/></geometry>
                    </collision>
                </link>
                <joint name="driver" type="revolute">
                    <parent link="base"/><child link="follower"/>
                    <limit lower="-1" upper="1" effort="4" velocity="2"/>
                </joint>
                <joint name="mimic_driver" type="continuous">
                    <parent link="follower"/><child link="tool"/>
                    <mimic joint="driver" multiplier="-2" offset="0.25"/>
                </joint>
                <link name="tool"/>
            </robot>"#,
        )
        .unwrap();
        assert_eq!(robot.joints[0].limit.as_ref().unwrap().lower, Some(-1.0));
        assert_eq!(robot.joints[0].limit.as_ref().unwrap().upper, Some(1.0));
        assert_eq!(robot.joints[1].mimic.as_ref().unwrap().joint, "driver");
        assert_eq!(robot.links[1].collisions[0].origin.xyz, [0.0, 0.0, 0.1]);
        assert_eq!(robot.links[1].collisions.len(), 1);
    }

    #[test]
    fn accepts_planar_and_floating_joints_with_preview_warnings() {
        let robot = parse_robot_description(
            r#"<robot name="floating">
                <link name="base"/>
                <link name="planar_link"/>
                <link name="floating_link"/>
                <joint name="planar" type="planar">
                    <parent link="base"/><child link="planar_link"/>
                    <axis xyz="1 0 0"/>
                </joint>
                <joint name="float" type="floating">
                    <parent link="base"/><child link="floating_link"/>
                </joint>
            </robot>"#,
        )
        .unwrap();
        assert_eq!(robot.joints.len(), 2);
        assert!(robot
            .warnings
            .iter()
            .any(|warning| warning.contains("planar") || warning.contains("floating")));
    }

    #[test]
    fn rejects_invalid_joint_limits_and_mimic_relationships() {
        let invalid_limit = r#"<robot name="test"><link name="a"/><link name="b"/><joint name="j" type="revolute"><parent link="a"/><child link="b"/><limit lower="2" upper="1"/></joint></robot>"#;
        assert!(parse_robot_description(invalid_limit).is_err());
        let missing_mimic = r#"<robot name="test"><link name="a"/><link name="b"/><joint name="j" type="revolute"><parent link="a"/><child link="b"/><mimic joint="missing"/></joint></robot>"#;
        assert!(parse_robot_description(missing_mimic).is_err());
        let mimic_cycle = r#"<robot name="test"><link name="a"/><link name="b"/><link name="c"/><joint name="one" type="revolute"><parent link="a"/><child link="b"/><mimic joint="two"/></joint><joint name="two" type="revolute"><parent link="b"/><child link="c"/><mimic joint="one"/></joint></robot>"#;
        assert!(parse_robot_description(mimic_cycle).is_err());
    }
}
