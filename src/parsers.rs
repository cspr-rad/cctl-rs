use super::{CasperNodePorts, CasperSidecarPorts, NodeState};

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{line_ending, multispace0, not_line_ending, space1},
    combinator::map,
    multi::separated_list0,
    sequence::tuple,
    IResult,
};

pub fn parse_node_state(input: &str) -> IResult<&str, NodeState> {
    alt((
        map(tag("RUNNING"), |_| NodeState::Running),
        map(tag("STOPPED"), |_| NodeState::Stopped),
    ))(input)
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RawNodeType {
    CasperNode(u8, u8, NodeState),
    CasperSidecar(u8, u8, NodeState),
}

pub fn parse_node_line(input: &str) -> IResult<&str, RawNodeType> {
    let (remainder, (_, _, group_id, _, node_id, _, status, _)) = tuple((
        multispace0,
        tag("validator-group-"),
        nom::character::complete::u8,
        tag(":cctl-node-"),
        nom::character::complete::u8,
        space1,
        parse_node_state,
        not_line_ending,
    ))(input)?;

    Ok((
        remainder,
        RawNodeType::CasperNode(group_id, node_id, status),
    ))
}

pub fn parse_sidecar_line(input: &str) -> IResult<&str, RawNodeType> {
    let (remainder, (_, _, group_id, _, node_id, _, _, status, _)) = tuple((
        multispace0,
        tag("validator-group-"),
        nom::character::complete::u8,
        tag(":cctl-node-"),
        nom::character::complete::u8,
        tag("-sidecar"),
        space1,
        parse_node_state,
        not_line_ending,
    ))(input)?;

    Ok((
        remainder,
        RawNodeType::CasperSidecar(group_id, node_id, status),
    ))
}

pub fn parse_cctl_infra_net_start_line(input: &str) -> IResult<&str, RawNodeType> {
    alt((parse_sidecar_line, parse_node_line))(input)
}

pub fn parse_cctl_infra_net_start_lines(input: &str) -> IResult<&str, Vec<RawNodeType>> {
    let (remainder, _) = nom::bytes::complete::take_until("validator-group")(input)?;
    separated_list0(tag("\n"), parse_cctl_infra_net_start_line)(remainder)
}

pub fn parse_cctl_infra_node_view_ports_node_id(input: &str) -> IResult<&str, u8> {
    let (remainder, (_, _, node_id)) = tuple((
        nom::bytes::complete::take_until("NODE-"),
        tag("NODE-"),
        nom::character::complete::u8,
    ))(input)?;
    Ok((remainder, node_id))
}

pub fn parse_cctl_infra_node_view_ports_port<'a>(
    port_type: &'a str,
) -> impl Fn(&'a str) -> IResult<&'a str, u16> {
    move |input: &str| {
        let (remainder, (_, _, _, port)) = tuple((
            nom::bytes::complete::take_until(port_type),
            nom::bytes::complete::take_until("-> "),
            tag("-> "),
            nom::character::complete::u16,
        ))(input)?;
        Ok((remainder, port))
    }
}

pub fn parse_cctl_infra_node_view_port_section(
    input: &str,
) -> IResult<&str, (u8, CasperNodePorts)> {
    let (remainder, (node_id, _, protocol_port, _, binary_port, _, rest_port, _, sse_port, _)) =
        tuple((
            parse_cctl_infra_node_view_ports_node_id,
            line_ending,
            parse_cctl_infra_node_view_ports_port("PROTOCOL"),
            line_ending,
            parse_cctl_infra_node_view_ports_port("BINARY"),
            line_ending,
            parse_cctl_infra_node_view_ports_port("REST"),
            line_ending,
            parse_cctl_infra_node_view_ports_port("SSE"),
            not_line_ending,
        ))(input)?;

    Ok((
        remainder,
        (
            node_id,
            CasperNodePorts {
                protocol_port,
                binary_port,
                rest_port,
                sse_port,
            },
        ),
    ))
}

pub fn parse_cctl_infra_node_view_port_lines(
    input: &str,
) -> IResult<&str, Vec<(u8, CasperNodePorts)>> {
    separated_list0(tag("\n"), parse_cctl_infra_node_view_port_section)(input)
}

pub fn parse_cctl_infra_sidecar_view_ports_node_id(input: &str) -> IResult<&str, u8> {
    let (remainder, (_, _, sidecar_id)) = tuple((
        nom::bytes::complete::take_until("SIDECAR-"),
        tag("SIDECAR-"),
        nom::character::complete::u8,
    ))(input)?;
    Ok((remainder, sidecar_id))
}

pub fn parse_cctl_infra_sidecar_view_port_section(
    input: &str,
) -> IResult<&str, (u8, CasperSidecarPorts)> {
    let (remainder, (sidecar_id, _, node_client_port, _, rpc_port, _, speculative_exec_port, _)) =
        tuple((
            parse_cctl_infra_sidecar_view_ports_node_id,
            line_ending,
            parse_cctl_infra_node_view_ports_port("NODE-CLIENT"),
            line_ending,
            parse_cctl_infra_node_view_ports_port("MAIN-RPC"),
            line_ending,
            parse_cctl_infra_node_view_ports_port("SPEC-EXEC"),
            not_line_ending,
        ))(input)?;

    Ok((
        remainder,
        (
            sidecar_id,
            CasperSidecarPorts {
                node_client_port,
                rpc_port,
                speculative_exec_port,
            },
        ),
    ))
}

pub fn parse_cctl_infra_sidecar_view_port_lines(
    input: &str,
) -> IResult<&str, Vec<(u8, CasperSidecarPorts)>> {
    separated_list0(tag("\n"), parse_cctl_infra_sidecar_view_port_section)(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Error;

    #[test]
    fn test_parse_node_line() -> Result<(), Error> {
        let input = "validator-group-1:cctl-node-1    RUNNING   pid 428229, uptime 0:09:06\n";
        let (_, parsed) = parse_node_line(input)?;
        Ok(assert_eq!(
            RawNodeType::CasperNode(1, 1, NodeState::Running),
            parsed
        ))
    }
    #[test]
    fn test_parse_sidecar_line() -> Result<(), Error> {
        let input =
            "validator-group-1:cctl-node-1-sidecar    RUNNING   pid 626096, uptime 0:00:03\n";
        let (_, parsed) = parse_sidecar_line(input)?;
        Ok(assert_eq!(
            RawNodeType::CasperSidecar(1, 1, NodeState::Running),
            parsed
        ))
    }

    #[test]
    fn test_parse_cctl_infra_net_start_lines() -> Result<(), Error> {
        let input = r#"
            2024-08-30T17:15:10.262713 [INFO] [626072] CCTL :: ---------------------------------------------------------------------------------
            2024-08-30T17:15:10.265112 [INFO] [626072] CCTL :: Network start begins
            2024-08-30T17:15:10.268196 [INFO] [626072] CCTL :: ---------------------------------------------------------------------------------
            2024-08-30T17:15:12.583839 [INFO] [626072] CCTL :: Daemon supervisor -> started
            2024-08-30T17:15:13.801089 [INFO] [626072] CCTL :: Genesis bootstrap nodes -> started
            2024-08-30T17:15:14.966443 [INFO] [626072] CCTL :: Genesis non-bootstrap nodes -> started
            validator-group-1:cctl-node-1            RUNNING   pid 626095, uptime 0:00:03
            validator-group-1:cctl-node-1-sidecar    RUNNING   pid 626096, uptime 0:00:03
            validator-group-1:cctl-node-2            RUNNING   pid 626097, uptime 0:00:03
            validator-group-1:cctl-node-2-sidecar    RUNNING   pid 626098, uptime 0:00:03
            validator-group-1:cctl-node-3            RUNNING   pid 626101, uptime 0:00:03
            validator-group-1:cctl-node-3-sidecar    RUNNING   pid 626102, uptime 0:00:03
            validator-group-2:cctl-node-4            RUNNING   pid 626285, uptime 0:00:02
            validator-group-2:cctl-node-4-sidecar    RUNNING   pid 626286, uptime 0:00:02
            validator-group-2:cctl-node-5            RUNNING   pid 626287, uptime 0:00:02
            validator-group-2:cctl-node-5-sidecar    RUNNING   pid 626288, uptime 0:00:02
            validator-group-3:cctl-node-10           STOPPED   Not started
            validator-group-3:cctl-node-10-sidecar   STOPPED   Not started
            validator-group-3:cctl-node-6            STOPPED   Not started
            validator-group-3:cctl-node-6-sidecar    STOPPED   Not started
            validator-group-3:cctl-node-7            STOPPED   Not started
            validator-group-3:cctl-node-7-sidecar    STOPPED   Not started
            validator-group-3:cctl-node-8            STOPPED   Not started
            validator-group-3:cctl-node-8-sidecar    STOPPED   Not started
            validator-group-3:cctl-node-9            STOPPED   Not started
            validator-group-3:cctl-node-9-sidecar    STOPPED   Not started
            2024-08-30T17:15:15.108318 [INFO] [626072] CCTL :: ---------------------------------------------------------------------------------
            2024-08-30T17:15:15.110310 [INFO] [626072] CCTL :: Network start ends
            2024-08-30T17:15:15.112531 [INFO] [626072] CCTL :: ---------------------------------------------------------------------------------
        "#;
        let (_, parsed) = parse_cctl_infra_net_start_lines(input)?;
        let expected = vec![
            RawNodeType::CasperNode(1, 1, NodeState::Running),
            RawNodeType::CasperSidecar(1, 1, NodeState::Running),
            RawNodeType::CasperNode(1, 2, NodeState::Running),
            RawNodeType::CasperSidecar(1, 2, NodeState::Running),
            RawNodeType::CasperNode(1, 3, NodeState::Running),
            RawNodeType::CasperSidecar(1, 3, NodeState::Running),
            RawNodeType::CasperNode(2, 4, NodeState::Running),
            RawNodeType::CasperSidecar(2, 4, NodeState::Running),
            RawNodeType::CasperNode(2, 5, NodeState::Running),
            RawNodeType::CasperSidecar(2, 5, NodeState::Running),
            RawNodeType::CasperNode(3, 10, NodeState::Stopped),
            RawNodeType::CasperSidecar(3, 10, NodeState::Stopped),
            RawNodeType::CasperNode(3, 6, NodeState::Stopped),
            RawNodeType::CasperSidecar(3, 6, NodeState::Stopped),
            RawNodeType::CasperNode(3, 7, NodeState::Stopped),
            RawNodeType::CasperSidecar(3, 7, NodeState::Stopped),
            RawNodeType::CasperNode(3, 8, NodeState::Stopped),
            RawNodeType::CasperSidecar(3, 8, NodeState::Stopped),
            RawNodeType::CasperNode(3, 9, NodeState::Stopped),
            RawNodeType::CasperSidecar(3, 9, NodeState::Stopped),
        ];
        Ok(assert_eq!(expected, parsed))
    }

    #[test]
    fn test_parse_cctl_infra_node_view_ports_node_id() -> Result<(), Error> {
        let input = "2024-09-02T08:44:46.871632 [INFO] [124520] CCTL :: NODE-1";
        let (_, parsed) = parse_cctl_infra_node_view_ports_node_id(input)?;
        Ok(assert_eq!(1, parsed))
    }

    #[test]
    fn test_parse_cctl_infra_node_view_ports_protocol() -> Result<(), Error> {
        let input = "2024-09-02T08:44:46.874259 [INFO] [124520] CCTL ::     PROTOCOL ----> 11101";
        let (_, parsed) = parse_cctl_infra_node_view_ports_port("PROTOCOL")(input)?;
        Ok(assert_eq!(11101, parsed))
    }

    #[test]
    fn test_parse_cctl_infra_node_view_ports_binary() -> Result<(), Error> {
        let input = "2024-09-02T08:44:46.876701 [INFO] [124520] CCTL ::     BINARY ------> 12101";
        let (_, parsed) = parse_cctl_infra_node_view_ports_port("BINARY")(input)?;
        Ok(assert_eq!(12101, parsed))
    }

    #[test]
    fn test_parse_cctl_infra_node_view_ports_rest() -> Result<(), Error> {
        let input = "2024-09-02T08:44:46.879103 [INFO] [124520] CCTL ::     REST --------> 13101";
        let (_, parsed) = parse_cctl_infra_node_view_ports_port("REST")(input)?;
        Ok(assert_eq!(13101, parsed))
    }

    #[test]
    fn test_parse_cctl_infra_node_view_ports_sse() -> Result<(), Error> {
        let input = "2024-09-02T08:44:46.881573 [INFO] [124520] CCTL ::     SSE ---------> 14101";
        let (_, parsed) = parse_cctl_infra_node_view_ports_port("SSE")(input)?;
        Ok(assert_eq!(14101, parsed))
    }

    #[test]
    fn test_parse_cctl_infra_node_view_port_node() -> Result<(), Error> {
        let input = r#"
            2024-09-02T08:44:46.871632 [INFO] [124520] CCTL :: NODE-1
            2024-09-02T08:44:46.874259 [INFO] [124520] CCTL ::     PROTOCOL ----> 11101
            2024-09-02T08:44:46.876701 [INFO] [124520] CCTL ::     BINARY ------> 12101
            2024-09-02T08:44:46.879103 [INFO] [124520] CCTL ::     REST --------> 13101
            2024-09-02T08:44:46.881573 [INFO] [124520] CCTL ::     SSE ---------> 14101
            2024-09-02T08:44:46.883303 [INFO] [124520] CCTL :: ------------------------------------------------------------------------------------------------------
        "#;
        let (_, parsed) = parse_cctl_infra_node_view_port_section(input)?;
        Ok(assert_eq!(
            (
                1,
                CasperNodePorts {
                    protocol_port: 11101,
                    binary_port: 12101,
                    rest_port: 13101,
                    sse_port: 14101,
                }
            ),
            parsed
        ))
    }

    #[test]
    fn test_parse_cctl_infra_node_view_port_lines() -> Result<(), Error> {
        let input = r#"
            2024-09-02T08:44:46.865013 [INFO] [124520] CCTL :: ------------------------------------------------------------------------------------------------------
            2024-09-02T08:44:46.871632 [INFO] [124520] CCTL :: NODE-1
            2024-09-02T08:44:46.874259 [INFO] [124520] CCTL ::     PROTOCOL ----> 11101
            2024-09-02T08:44:46.876701 [INFO] [124520] CCTL ::     BINARY ------> 12101
            2024-09-02T08:44:46.879103 [INFO] [124520] CCTL ::     REST --------> 13101
            2024-09-02T08:44:46.881573 [INFO] [124520] CCTL ::     SSE ---------> 14101
            2024-09-02T08:44:46.883303 [INFO] [124520] CCTL :: ------------------------------------------------------------------------------------------------------
            2024-09-02T08:44:46.889475 [INFO] [124520] CCTL :: NODE-2
            2024-09-02T08:44:46.891804 [INFO] [124520] CCTL ::     PROTOCOL ----> 11102
            2024-09-02T08:44:46.894156 [INFO] [124520] CCTL ::     BINARY ------> 12102
            2024-09-02T08:44:46.896950 [INFO] [124520] CCTL ::     REST --------> 13102
            2024-09-02T08:44:46.899175 [INFO] [124520] CCTL ::     SSE ---------> 14102
            2024-09-02T08:44:46.901358 [INFO] [124520] CCTL :: ------------------------------------------------------------------------------------------------------
            2024-09-02T08:44:46.906934 [INFO] [124520] CCTL :: NODE-3
            2024-09-02T08:44:46.909238 [INFO] [124520] CCTL ::     PROTOCOL ----> 11103
            2024-09-02T08:44:46.911518 [INFO] [124520] CCTL ::     BINARY ------> 12103
            2024-09-02T08:44:46.913895 [INFO] [124520] CCTL ::     REST --------> 13103
            2024-09-02T08:44:46.916383 [INFO] [124520] CCTL ::     SSE ---------> 14103
            2024-09-02T08:44:46.918830 [INFO] [124520] CCTL :: ------------------------------------------------------------------------------------------------------
            2024-09-02T08:44:46.925381 [INFO] [124520] CCTL :: NODE-4
            2024-09-02T08:44:46.927266 [INFO] [124520] CCTL ::     PROTOCOL ----> 11104
            2024-09-02T08:44:46.929719 [INFO] [124520] CCTL ::     BINARY ------> 12104
            2024-09-02T08:44:46.932628 [INFO] [124520] CCTL ::     REST --------> 13104
            2024-09-02T08:44:46.934868 [INFO] [124520] CCTL ::     SSE ---------> 14104
            2024-09-02T08:44:46.937664 [INFO] [124520] CCTL :: ------------------------------------------------------------------------------------------------------
            2024-09-02T08:44:46.943128 [INFO] [124520] CCTL :: NODE-5
            2024-09-02T08:44:46.945253 [INFO] [124520] CCTL ::     PROTOCOL ----> 11105
            2024-09-02T08:44:46.947316 [INFO] [124520] CCTL ::     BINARY ------> 12105
            2024-09-02T08:44:46.949464 [INFO] [124520] CCTL ::     REST --------> 13105
            2024-09-02T08:44:46.951770 [INFO] [124520] CCTL ::     SSE ---------> 14105
            2024-09-02T08:44:46.954542 [INFO] [124520] CCTL :: ------------------------------------------------------------------------------------------------------
        "#;
        let (_, parsed) = parse_cctl_infra_node_view_port_lines(input)?;
        let expected = vec![
            (
                1,
                CasperNodePorts {
                    protocol_port: 11101,
                    binary_port: 12101,
                    rest_port: 13101,
                    sse_port: 14101,
                },
            ),
            (
                2,
                CasperNodePorts {
                    protocol_port: 11102,
                    binary_port: 12102,
                    rest_port: 13102,
                    sse_port: 14102,
                },
            ),
            (
                3,
                CasperNodePorts {
                    protocol_port: 11103,
                    binary_port: 12103,
                    rest_port: 13103,
                    sse_port: 14103,
                },
            ),
            (
                4,
                CasperNodePorts {
                    protocol_port: 11104,
                    binary_port: 12104,
                    rest_port: 13104,
                    sse_port: 14104,
                },
            ),
            (
                5,
                CasperNodePorts {
                    protocol_port: 11105,
                    binary_port: 12105,
                    rest_port: 13105,
                    sse_port: 14105,
                },
            ),
        ];
        Ok(assert_eq!(expected, parsed))
    }

    #[test]
    fn test_parse_cctl_infra_sidecar_view_port_node() -> Result<(), Error> {
        let input = r#"
            2024-09-02T09:49:32.804362 [INFO] [194431] CCTL :: SIDECAR-1
            2024-09-02T09:49:32.807243 [INFO] [194431] CCTL ::     NODE-CLIENT -> 12101
            2024-09-02T09:49:32.809625 [INFO] [194431] CCTL ::     MAIN-RPC ----> 21101
            2024-09-02T09:49:32.811288 [INFO] [194431] CCTL ::     SPEC-EXEC ---> 22101
        "#;
        let (_, parsed) = parse_cctl_infra_sidecar_view_port_section(input)?;
        Ok(assert_eq!(
            (
                1,
                CasperSidecarPorts {
                    node_client_port: 12101,
                    rpc_port: 21101,
                    speculative_exec_port: 22101,
                }
            ),
            parsed
        ))
    }

    #[test]
    fn test_parse_cctl_infra_sidecar_view_port_lines() -> Result<(), Error> {
        let input = r#"
            2024-09-02T09:49:32.792987 [INFO] [194431] CCTL :: ------------------------------------------------------------------------------------------------------
            2024-09-02T09:49:32.794753 [INFO] [194431] CCTL :: SIDECAR PORTS
            2024-09-02T09:49:32.796968 [INFO] [194431] CCTL :: ------------------------------------------------------------------------------------------------------
            2024-09-02T09:49:32.804362 [INFO] [194431] CCTL :: SIDECAR-1
            2024-09-02T09:49:32.807243 [INFO] [194431] CCTL ::     NODE-CLIENT -> 12101
            2024-09-02T09:49:32.809625 [INFO] [194431] CCTL ::     MAIN-RPC ----> 21101
            2024-09-02T09:49:32.811288 [INFO] [194431] CCTL ::     SPEC-EXEC ---> 22101
            2024-09-02T09:49:32.816160 [INFO] [194431] CCTL :: SIDECAR-2
            2024-09-02T09:49:32.818236 [INFO] [194431] CCTL ::     NODE-CLIENT -> 12102
            2024-09-02T09:49:32.820258 [INFO] [194431] CCTL ::     MAIN-RPC ----> 21102
            2024-09-02T09:49:32.822750 [INFO] [194431] CCTL ::     SPEC-EXEC ---> 22102
            2024-09-02T09:49:32.827580 [INFO] [194431] CCTL :: SIDECAR-3
            2024-09-02T09:49:32.829656 [INFO] [194431] CCTL ::     NODE-CLIENT -> 12103
            2024-09-02T09:49:32.831461 [INFO] [194431] CCTL ::     MAIN-RPC ----> 21103
            2024-09-02T09:49:32.833187 [INFO] [194431] CCTL ::     SPEC-EXEC ---> 22103
            2024-09-02T09:49:32.837294 [INFO] [194431] CCTL :: SIDECAR-4
            2024-09-02T09:49:32.839495 [INFO] [194431] CCTL ::     NODE-CLIENT -> 12104
            2024-09-02T09:49:32.841550 [INFO] [194431] CCTL ::     MAIN-RPC ----> 21104
            2024-09-02T09:49:32.843292 [INFO] [194431] CCTL ::     SPEC-EXEC ---> 22104
            2024-09-02T09:49:32.847555 [INFO] [194431] CCTL :: SIDECAR-5
            2024-09-02T09:49:32.849820 [INFO] [194431] CCTL ::     NODE-CLIENT -> 12105
            2024-09-02T09:49:32.852074 [INFO] [194431] CCTL ::     MAIN-RPC ----> 21105
            2024-09-02T09:49:32.853727 [INFO] [194431] CCTL ::     SPEC-EXEC ---> 22105
        "#;
        let (_, parsed) = parse_cctl_infra_sidecar_view_port_lines(input)?;
        let expected = vec![
            (
                1,
                CasperSidecarPorts {
                    node_client_port: 12101,
                    rpc_port: 21101,
                    speculative_exec_port: 22101,
                },
            ),
            (
                2,
                CasperSidecarPorts {
                    node_client_port: 12102,
                    rpc_port: 21102,
                    speculative_exec_port: 22102,
                },
            ),
            (
                3,
                CasperSidecarPorts {
                    node_client_port: 12103,
                    rpc_port: 21103,
                    speculative_exec_port: 22103,
                },
            ),
            (
                4,
                CasperSidecarPorts {
                    node_client_port: 12104,
                    rpc_port: 21104,
                    speculative_exec_port: 22104,
                },
            ),
            (
                5,
                CasperSidecarPorts {
                    node_client_port: 12105,
                    rpc_port: 21105,
                    speculative_exec_port: 22105,
                },
            ),
        ];
        Ok(assert_eq!(expected, parsed))
    }
}
