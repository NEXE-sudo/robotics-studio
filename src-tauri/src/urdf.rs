use serde::Serialize;
use std::collections::{HashMap, HashSet};

const MAX_DESCRIPTION_BYTES: usize = 2 * 1024 * 1024;
const MAX_LINKS: usize = 512;
const MAX_JOINTS: usize = 512;
const MAX_VISUALS_PER_LINK: usize = 64;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotOrigin {
    pub xyz: [f64; 3],
    pub rpy: [f64; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RobotGeometry {
    Box { size: [f64; 3] },
    Cylinder { radius: f64, length: f64 },
    Sphere { radius: f64 },
    Unsupported { description: String },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotVisual {
    pub origin: RobotOrigin,
    pub geometry: RobotGeometry,
    pub color: Option<[f64; 4]>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotLink {
    pub name: String,
    pub visuals: Vec<RobotVisual>,
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
        "mesh" => Ok(RobotGeometry::Unsupported {
            description: attributes
                .get("filename")
                .cloned()
                .unwrap_or_else(|| "mesh with no filename".to_string()),
        }),
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
    let mut links = Vec::new();
    let mut joints = Vec::new();
    let mut warnings = Vec::new();

    loop {
        match reader.read_event() {
            Ok(
                event @ (quick_xml::events::Event::Start(_) | quick_xml::events::Event::Empty(_)),
            ) => {
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
                        });
                    }
                    "origin" if parent == Some("visual") => {
                        if let Some(visual) = current_visual.as_mut() {
                            visual.origin = parse_origin(&attributes)?;
                        }
                    }
                    "origin" if parent == Some("joint") => {
                        if let Some(joint) = current_joint.as_mut() {
                            joint.origin = parse_origin(&attributes)?;
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
                    "color" if parent == Some("material") => {
                        if let (Some(visual), Some(color)) =
                            (current_visual.as_mut(), attributes.get("rgba"))
                        {
                            visual.color = Some(parse_color(color)?);
                        }
                    }
                    "box" | "cylinder" | "sphere" | "mesh" if parent == Some("geometry") => {
                        if let Some(visual) = current_visual.as_mut() {
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
                        "visual" => {
                            return Err("URDF visual is missing a geometry".to_string());
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
                                "Joint '{}' uses unsupported '{}' motion and remains fixed",
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
    use super::{parse_robot_description, RobotGeometry};

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
    fn reports_mesh_geometry_as_unsupported_instead_of_silently_rendering_it() {
        let robot = parse_robot_description(
            r#"<robot name="test"><link name="base"><visual><geometry><mesh filename="package://robot/meshes/base.stl"/></geometry></visual></link></robot>"#,
        )
        .unwrap();

        assert!(matches!(
            robot.links[0].visuals[0].geometry,
            RobotGeometry::Unsupported { .. }
        ));
        assert_eq!(robot.warnings.len(), 1);
        assert!(robot.warnings[0].contains("base.stl"));
    }
}
