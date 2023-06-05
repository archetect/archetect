use crate::mermaid::erdiagram::Cardinality::{ExactlyOne, OneOrMore, ZeroOrMore, ZeroOrOne};
use crate::mermaid::erdiagram::Identification::{Identifying, NonIdentifying};
use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case, take_till};
use nom::character::complete::{alpha1, alphanumeric1, char, multispace0, space0};
use nom::character::is_newline;
use nom::combinator::{map, opt, recognize, value};
use nom::multi::{many0, many0_count, separated_list0};
use nom::sequence::{delimited, pair, tuple};
use nom::IResult;

#[derive(Clone, Debug, PartialOrd, PartialEq)]
enum DiagramType {
    EntityRelationship,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
struct MermaidErDiagram<'i> {
    sections: Vec<DiagramSection<'i>>,
}

fn mermaid_er_diagram(input: &str) -> IResult<&str, MermaidErDiagram> {
    let (remaining, parsed) = tuple((delimited(multispace0, tag("erDiagram"), multispace0), diagram_sections))(input)?;
    Ok((remaining, MermaidErDiagram { sections: parsed.1 }))
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
enum DiagramSection<'i> {
    EntityRelation(EntityRelation<'i>),
    EntityAttributes(EntityAttributes<'i>),
    Entity(Entity<'i>),
}

fn diagram_section(input: &str) -> IResult<&str, DiagramSection> {
    alt((
        map(entity_relation, DiagramSection::EntityRelation),
        map(entity_attributes, DiagramSection::EntityAttributes),
        map(entity, DiagramSection::Entity),
    ))(input)
}

fn diagram_sections(input: &str) -> IResult<&str, Vec<DiagramSection>> {
    many0(diagram_section)(input)
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
struct Entity<'i>(&'i str);

#[derive(Clone, Debug, PartialOrd, PartialEq)]
struct Label<'i>(&'i str);

#[derive(Clone, Debug, PartialOrd, PartialEq)]
struct EntityRelation<'i> {
    left: Entity<'i>,
    right: Entity<'i>,
    label: Label<'i>,
    relationship: Relationship,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
struct Relationship {
    left_cardinality: Cardinality,
    identification: Identification,
    right_cardinality: Cardinality,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
enum Cardinality {
    ZeroOrOne,
    ZeroOrMore,
    ExactlyOne,
    OneOrMore,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
enum KeyConstraint {
    PK,
    FK,
    UK,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
enum Identification {
    Identifying,
    NonIdentifying,
}

fn entity(input: &str) -> IResult<&str, Entity> {
    let (remaining, parsed) = delimited(multispace0, quotable_identifier, multispace0)(input)?;
    Ok((remaining, Entity(parsed)))
}

fn label(input: &str) -> IResult<&str, Label> {
    let (remaining, parsed) = delimited(space0, quotable_identifier, space0)(input)?;
    Ok((remaining, Label(parsed)))
}

fn key_constraint(input: &str) -> IResult<&str, KeyConstraint> {
    delimited(
        multispace0,
        alt((
            value(KeyConstraint::PK, tag_no_case("PK")),
            value(KeyConstraint::FK, tag_no_case("FK")),
            value(KeyConstraint::UK, tag_no_case("UK")),
        )),
        multispace0,
    )(input)
}

fn key_constraints(input: &str) -> IResult<&str, Vec<KeyConstraint>> {
    separated_list0(char(','), key_constraint)(input)
}

fn unquoted_identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(alpha1, many0_count(alt((alphanumeric1, tag("_"), tag("-"))))))(input)
}

fn quoted_string(input: &str) -> IResult<&str, &str> {
    delimited(tag("\""), take_till(|c| is_newline(c as u8) || c == '"'), tag("\""))(input)
}

fn quotable_identifier(input: &str) -> IResult<&str, &str> {
    alt((quoted_string, unquoted_identifier))(input)
}

fn left_cardinality(input: &str) -> IResult<&str, Cardinality> {
    alt((
        value(ZeroOrOne, tag("|o")),
        value(ZeroOrOne, tag("one or zero")),
        value(ZeroOrOne, tag("zero or one")),
        value(ZeroOrMore, tag("}o")),
        value(ZeroOrMore, tag("zero or more")),
        value(ZeroOrMore, tag("zero or many")),
        value(ZeroOrMore, tag("many(0)")),
        value(ZeroOrMore, tag("0+")),
        value(OneOrMore, tag("}|")),
        value(OneOrMore, tag("one or more")),
        value(OneOrMore, tag("one or many")),
        value(OneOrMore, tag("many(1)")),
        value(OneOrMore, tag("1+")),
        value(ExactlyOne, tag("||")),
        value(ExactlyOne, tag("only one")),
        value(ExactlyOne, tag("1")),
        value(ZeroOrMore, tag("many")),
        value(ExactlyOne, tag("one")),
    ))(input)
}

fn right_cardinality(input: &str) -> IResult<&str, Cardinality> {
    alt((
        value(ZeroOrOne, tag("o|")),
        value(ZeroOrOne, tag("one or zero")),
        value(ZeroOrOne, tag("zero or one")),
        value(ZeroOrMore, tag("o{")),
        value(ZeroOrMore, tag("zero or more")),
        value(ZeroOrMore, tag("zero or many")),
        value(ZeroOrMore, tag("many(0)")),
        value(ZeroOrMore, tag("0+")),
        value(OneOrMore, tag("|{")),
        value(OneOrMore, tag("one or more")),
        value(OneOrMore, tag("one or many")),
        value(OneOrMore, tag("many(1)")),
        value(OneOrMore, tag("1+")),
        value(ExactlyOne, tag("||")),
        value(ExactlyOne, tag("only one")),
        value(ExactlyOne, tag("1")),
        value(ZeroOrMore, tag("many")),
        value(ExactlyOne, tag("one")),
    ))(input)
}

fn identification(input: &str) -> IResult<&str, Identification> {
    alt((
        value(Identifying, tag("--")),
        value(Identifying, tag("to")),
        value(NonIdentifying, tag("..")),
        value(NonIdentifying, tag(".-")),
        value(NonIdentifying, tag("-.")),
        value(NonIdentifying, tag("optionally to")),
    ))(input)
}

fn entity_relation(input: &str) -> IResult<&str, EntityRelation> {
    let (remaining, parsed) = tuple((
        entity,                              // left: 0
        relationship,                        // 1
        entity,                              // right: 2
        delimited(space0, tag(":"), space0), // 3
        label,                               // label: 4
    ))(input)?;
    Ok((
        remaining,
        EntityRelation {
            left: parsed.0,
            right: parsed.2,
            label: parsed.4,
            relationship: parsed.1,
        },
    ))
}

fn relationship(input: &str) -> IResult<&str, Relationship> {
    let (remaining, parsed) = tuple((
        left_cardinality,  // 0
        multispace0,       // 1
        identification,    // 2
        multispace0,       // 3
        right_cardinality, // 4
    ))(input)?;
    Ok((
        remaining,
        Relationship {
            left_cardinality: parsed.0,
            identification: parsed.2,
            right_cardinality: parsed.4,
        },
    ))
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
struct Type<'i>(&'i str);

fn attribute_type(input: &str) -> IResult<&str, Type> {
    let (remaining, parsed) = recognize(pair(
        alpha1,
        many0_count(alt((
            alphanumeric1,
            tag("_"),
            tag("-"),
            tag("("),
            tag(")"),
            tag("["),
            tag("]"),
        ))),
    ))(input)?;
    Ok((remaining, Type(parsed)))
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
struct Attribute<'i> {
    typ: Type<'i>,
    name: &'i str,
    constraints: Vec<KeyConstraint>,
    comment: Option<&'i str>,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
struct EntityAttributes<'i> {
    name: Entity<'i>,
    attributes: Vec<Attribute<'i>>,
}

fn attribute_spec(input: &str) -> IResult<&str, Attribute> {
    let (remaining, parsed) = tuple((
        attribute_type,
        delimited(multispace0, unquoted_identifier, multispace0),
        key_constraints,
        opt(quoted_string),
    ))(input)?;
    Ok((
        remaining,
        Attribute {
            typ: parsed.0,
            name: parsed.1,
            constraints: parsed.2,
            comment: parsed.3,
        },
    ))
}

fn entity_attributes(input: &str) -> IResult<&str, EntityAttributes> {
    let (remaining, parsed) = tuple((
        entity,
        delimited(multispace0, tag("{"), multispace0),
        many0(delimited(multispace0, attribute_spec, multispace0)),
        delimited(multispace0, tag("}"), multispace0),
    ))(input)?;
    Ok((
        remaining,
        EntityAttributes {
            name: parsed.0,
            attributes: parsed.2,
        },
    ))
}

fn parse_declaration(_input: &str) -> IResult<&str, &str> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::erdiagram::KeyConstraint::{FK, PK, UK};
    use Cardinality::ExactlyOne;
    use Identification::Identifying;

    #[test]
    fn test_parse_left_cardinality() {
        assert_eq!(left_cardinality("|o to o|"), Ok((" to o|", ZeroOrOne)));
        assert_eq!(left_cardinality("one or zero to o|"), Ok((" to o|", ZeroOrOne)));
        assert_eq!(left_cardinality("zero or one to o|"), Ok((" to o|", ZeroOrOne)));
    }

    #[test]
    fn test_parse_quoted_identifier() {
        assert_eq!(quoted_string("\"i\""), Ok(("", "i")));
    }

    #[test]
    fn test_parse_relationship() {
        assert_eq!(
            relationship("|o to o|"),
            Ok((
                "",
                Relationship {
                    left_cardinality: ZeroOrOne,
                    identification: Identifying,
                    right_cardinality: ZeroOrOne,
                }
            ))
        );

        assert_eq!(
            relationship("|o .. o|"),
            Ok((
                "",
                Relationship {
                    left_cardinality: ZeroOrOne,
                    identification: NonIdentifying,
                    right_cardinality: ZeroOrOne,
                }
            ))
        );

        assert_eq!(
            relationship("|o -- o|"),
            Ok((
                "",
                Relationship {
                    left_cardinality: ZeroOrOne,
                    identification: Identifying,
                    right_cardinality: ZeroOrOne,
                }
            ))
        );

        assert_eq!(
            relationship("one or zero to zero or one"),
            Ok((
                "",
                Relationship {
                    left_cardinality: ZeroOrOne,
                    identification: Identifying,
                    right_cardinality: ZeroOrOne,
                }
            ))
        );

        assert_eq!(
            relationship("zero or one optionally to one or zero"),
            Ok((
                "",
                Relationship {
                    left_cardinality: ZeroOrOne,
                    identification: NonIdentifying,
                    right_cardinality: ZeroOrOne,
                }
            ))
        );
    }

    #[test]
    fn test_parse_entity_relation() {
        assert_eq!(
            entity_relation("\"CAR\" }o--|| \"NAMED-DRIVER\" : \"foo\""),
            Ok((
                "",
                EntityRelation {
                    left: Entity("CAR"),
                    right: Entity("NAMED-DRIVER"),
                    label: Label("foo"),
                    relationship: Relationship {
                        left_cardinality: ZeroOrMore,
                        identification: Identifying,
                        right_cardinality: ExactlyOne,
                    },
                }
            ))
        );

        assert_eq!(
            entity_relation("PERSON 0+ to |{ NAMED-DRIVER : is"),
            Ok((
                "",
                EntityRelation {
                    left: Entity("PERSON"),
                    right: Entity("NAMED-DRIVER"),
                    label: Label("is"),
                    relationship: Relationship {
                        left_cardinality: ZeroOrMore,
                        identification: Identifying,
                        right_cardinality: OneOrMore,
                    },
                }
            ))
        );

        assert_eq!(
            entity_relation(r#"CUSTOMER ||--o{ INVOICE : "places""#),
            Ok((
                "",
                EntityRelation {
                    left: Entity("CUSTOMER"),
                    right: Entity("INVOICE"),
                    label: Label("places"),
                    relationship: Relationship {
                        left_cardinality: ExactlyOne,
                        identification: Identifying,
                        right_cardinality: ZeroOrMore,
                    },
                }
            ))
        );
    }

    #[test]
    fn test_parse_key_constraint() {
        assert_eq!(key_constraint("FK"), Ok(("", FK)));
        assert_eq!(key_constraint("\nFK\t"), Ok(("", FK)));
        assert_eq!(key_constraint(" PK"), Ok(("", PK)));
        assert_eq!(key_constraint("pK "), Ok(("", PK)));
        assert_eq!(key_constraint(" pk "), Ok(("", PK)));
    }

    #[test]
    fn test_parse_key_constraints() {
        assert_eq!(key_constraints(""), Ok(("", Vec::new())));
        assert_eq!(key_constraints("FK"), Ok(("", vec![FK])));
        assert_eq!(key_constraints("FK,\nPK"), Ok(("", vec![FK, PK])));
    }

    #[test]
    fn test_parse_attribute_spec() {
        assert_eq!(
            attribute_spec("date payment_date"),
            Ok((
                "",
                Attribute {
                    typ: Type("date"),
                    name: "payment_date",
                    constraints: vec![],
                    comment: None,
                }
            ))
        )
    }

    #[test]
    fn test_parse_attributes_section() {
        assert_eq!(
            entity_attributes("CAR { string name }"),
            Ok((
                "",
                EntityAttributes {
                    name: Entity("CAR"),
                    attributes: vec![Attribute {
                        typ: Type("string"),
                        name: "name",
                        constraints: vec![],
                        comment: None,
                    }],
                }
            ))
        );

        assert_eq!(
            entity_attributes(
                r#"CAR
{
    int id FK, PK "Primary Key"
    string name
    int age
}
        "#
            ),
            Ok((
                "",
                EntityAttributes {
                    name: Entity("CAR"),
                    attributes: vec![
                        Attribute {
                            typ: Type("int"),
                            name: "id",
                            constraints: vec![FK, PK],
                            comment: Some("Primary Key"),
                        },
                        Attribute {
                            typ: Type("string"),
                            name: "name",
                            constraints: vec![],
                            comment: None,
                        },
                        Attribute {
                            typ: Type("int"),
                            name: "age",
                            constraints: vec![],
                            comment: None,
                        },
                    ],
                }
            ))
        );

        assert_eq!(
            entity_attributes(
                r#""01 TRUCK"
{
    int id UK "Primary Key"
    string name "User Name" string[] components
}
        "#
            ),
            Ok((
                "",
                EntityAttributes {
                    name: Entity("01 TRUCK"),
                    attributes: vec![
                        Attribute {
                            typ: Type("int"),
                            name: "id",
                            constraints: vec![UK],
                            comment: Some("Primary Key"),
                        },
                        Attribute {
                            typ: Type("string"),
                            name: "name",
                            constraints: vec![],
                            comment: Some("User Name"),
                        },
                        Attribute {
                            typ: Type("string[]"),
                            name: "components",
                            constraints: vec![],
                            comment: None,
                        },
                    ],
                }
            ))
        );

        assert_eq!(
            entity_attributes(r#""01 TRUCK" { int id UK "Primary Key" string name "User Name" string[] components } "#),
            Ok((
                "",
                EntityAttributes {
                    name: Entity("01 TRUCK"),
                    attributes: vec![
                        Attribute {
                            typ: Type("int"),
                            name: "id",
                            constraints: vec![UK],
                            comment: Some("Primary Key"),
                        },
                        Attribute {
                            typ: Type("string"),
                            name: "name",
                            constraints: vec![],
                            comment: Some("User Name"),
                        },
                        Attribute {
                            typ: Type("string[]"),
                            name: "components",
                            constraints: vec![],
                            comment: None,
                        },
                    ],
                }
            ))
        );
    }

    #[test]
    fn test_parse_diagram_section() {
        assert_eq!(
            diagram_section("CAR { string name }"),
            Ok((
                "",
                DiagramSection::EntityAttributes(EntityAttributes {
                    name: Entity("CAR"),
                    attributes: vec![Attribute {
                        typ: Type("string"),
                        name: "name",
                        constraints: vec![],
                        comment: None,
                    }],
                })
            ))
        );
    }

    #[test]
    fn test_parse_diagram_sections() {
        assert_eq!(
            diagram_sections(
                r#"
CUSTOMER
CUSTOMER ||--o{ INVOICE : "places"
CUSTOMER {
  int id
  string name
  string email
  string address
}
"#
            ),
            Ok((
                "",
                vec![
                    DiagramSection::Entity(Entity("CUSTOMER")),
                    DiagramSection::EntityRelation(EntityRelation {
                        left: Entity("CUSTOMER"),
                        right: Entity("INVOICE"),
                        label: Label("places"),
                        relationship: Relationship {
                            left_cardinality: ExactlyOne,
                            identification: Identifying,
                            right_cardinality: ZeroOrMore,
                        },
                    }),
                    DiagramSection::EntityAttributes(EntityAttributes {
                        name: Entity("CUSTOMER"),
                        attributes: vec![
                            Attribute {
                                typ: Type("int"),
                                name: "id",
                                constraints: vec![],
                                comment: None,
                            },
                            Attribute {
                                typ: Type("string"),
                                name: "name",
                                constraints: vec![],
                                comment: None,
                            },
                            Attribute {
                                typ: Type("string"),
                                name: "email",
                                constraints: vec![],
                                comment: None,
                            },
                            Attribute {
                                typ: Type("string"),
                                name: "address",
                                constraints: vec![],
                                comment: None,
                            },
                        ],
                    })
                ]
            ))
        );
    }

    #[test]
    fn test_parse_er_diagram() {
        assert_eq!(
            mermaid_er_diagram(
                r#"
erDiagram
    CUSTOMER }|..|{ DELIVERY-ADDRESS : has
    CUSTOMER {
      int id
      string name
      string email
      string address
    }
    CUSTOMER ||--o{ ORDER : places
    CUSTOMER ||--o{ INVOICE : "liable for"
    DELIVERY-ADDRESS ||--o{ ORDER : receives
    INVOICE ||--|{ ORDER : covers"#
            ),
            Ok((
                "",
                MermaidErDiagram {
                    sections: vec![
                        DiagramSection::EntityRelation(EntityRelation {
                            left: Entity("CUSTOMER"),
                            right: Entity("DELIVERY-ADDRESS"),
                            label: Label("has"),
                            relationship: Relationship {
                                left_cardinality: OneOrMore,
                                identification: NonIdentifying,
                                right_cardinality: OneOrMore,
                            },
                        }),
                        DiagramSection::EntityAttributes(EntityAttributes {
                            name: Entity("CUSTOMER"),
                            attributes: vec![
                                Attribute {
                                    typ: Type("int"),
                                    name: "id",
                                    constraints: vec![],
                                    comment: None,
                                },
                                Attribute {
                                    typ: Type("string"),
                                    name: "name",
                                    constraints: vec![],
                                    comment: None,
                                },
                                Attribute {
                                    typ: Type("string"),
                                    name: "email",
                                    constraints: vec![],
                                    comment: None,
                                },
                                Attribute {
                                    typ: Type("string"),
                                    name: "address",
                                    constraints: vec![],
                                    comment: None,
                                },
                            ],
                        }),
                        DiagramSection::EntityRelation(EntityRelation {
                            left: Entity("CUSTOMER"),
                            right: Entity("ORDER"),
                            label: Label("places"),
                            relationship: Relationship {
                                left_cardinality: ExactlyOne,
                                identification: Identifying,
                                right_cardinality: ZeroOrMore,
                            },
                        }),
                        DiagramSection::EntityRelation(EntityRelation {
                            left: Entity("CUSTOMER"),
                            right: Entity("INVOICE"),
                            label: Label("liable for"),
                            relationship: Relationship {
                                left_cardinality: ExactlyOne,
                                identification: Identifying,
                                right_cardinality: ZeroOrMore,
                            },
                        }),
                        DiagramSection::EntityRelation(EntityRelation {
                            left: Entity("DELIVERY-ADDRESS"),
                            right: Entity("ORDER"),
                            label: Label("receives"),
                            relationship: Relationship {
                                left_cardinality: ExactlyOne,
                                identification: Identifying,
                                right_cardinality: ZeroOrMore,
                            },
                        }),
                        DiagramSection::EntityRelation(EntityRelation {
                            left: Entity("INVOICE"),
                            right: Entity("ORDER"),
                            label: Label("covers"),
                            relationship: Relationship {
                                left_cardinality: ExactlyOne,
                                identification: Identifying,
                                right_cardinality: OneOrMore,
                            },
                        }),]})));
    }

    #[test]
    fn test_chatgpt_diagram() {
        assert!(mermaid_er_diagram(r#"
erDiagram
    USER ||--o{ PROFILE : has
    USER ||--o{ ROLE : has
    USER ||--o{ PERMISSION : has
    ROLE ||--o{ PERMISSION : contains

    USER {
        string username
        string password
        string email
        datetime last_login
    }

    PROFILE {
        string full_name
        string avatar_url
        date birth_date
    }

    ROLE {
        string name
        string description
    }

    PERMISSION {
        string name
        string description
    }

        "#).is_ok());
    }
}
