use {
    crate::{
        header::{ControlMessageType, DataMessageType, Header, MessageType},
        pdo::{
            AugmentedPowerDataObject, AugmentedPowerDataObjectRaw, Battery,
            EPRAdjustableVoltageSupply, FixedSupply, FixedVariableRequestDataObject,
            PowerDataObject, PowerDataObjectRaw, RequestDataObject, SPRProgrammablePowerSupply,
            VDMHeader, VDMHeaderRaw, VDMHeaderStructured, VDMHeaderUnstructured, VDMType,
            VariableSupply,
        },
    },
    byteorder::{ByteOrder, LittleEndian},
    defmt::{trace, warn, Format},
    heapless::Vec,
};

#[derive(Debug, Clone, Format)]
pub enum Message {
    Accept,
    Reject,
    Ready,
    SourceCapabilities(Vec<PowerDataObject, 8>),
    Request(RequestDataObject),
    VendorDefined(VDMHeader), // TODO: Incomplete
    SoftReset,
    Unknown,
}

impl Message {
    pub fn parse(header: Header, payload: &[u8]) -> Self {
        match header.message_type() {
            MessageType::Control(ControlMessageType::Accept) => Message::Accept,
            MessageType::Control(ControlMessageType::Reject) => Message::Reject,
            MessageType::Control(ControlMessageType::PsRdy) => Message::Ready,
            MessageType::Control(ControlMessageType::SoftReset) => Message::SoftReset,
            MessageType::Data(DataMessageType::SourceCapabilities) => Message::SourceCapabilities(
                payload
                    .chunks_exact(4)
                    .take(header.num_objects())
                    .map(|buf| PowerDataObjectRaw(LittleEndian::read_u32(buf)))
                    .map(|pdo| match pdo.kind() {
                        0b00 => PowerDataObject::FixedSupply(FixedSupply(pdo.0)),
                        0b01 => PowerDataObject::Battery(Battery(pdo.0)),
                        0b10 => PowerDataObject::VariableSupply(VariableSupply(pdo.0)),
                        0b11 => PowerDataObject::AugmentedPowerDataObject({
                            match AugmentedPowerDataObjectRaw(pdo.0).supply() {
                                0b00 => {
                                    AugmentedPowerDataObject::SPR(SPRProgrammablePowerSupply(pdo.0))
                                }
                                0b01 => {
                                    AugmentedPowerDataObject::EPR(EPRAdjustableVoltageSupply(pdo.0))
                                }
                                _ => unreachable!(),
                            }
                        }),
                        _ => unreachable!(),
                    })
                    .collect(),
            ),
            MessageType::Data(DataMessageType::Request) => {
                Message::Request(RequestDataObject::FixedSupply(
                    FixedVariableRequestDataObject(LittleEndian::read_u32(payload)),
                ))
            }
            MessageType::Data(DataMessageType::VendorDefined) => {
                // Keep for now...
                // let len = payload.len();
                // let num_obj = header.num_objects();
                //debug!("VENDOR: {:?}, {:?}, {:?}", len, num_obj, payload);

                let header = {
                    let raw = VDMHeaderRaw(LittleEndian::read_u32(&payload[..4]));
                    match raw.vdm_type() {
                        VDMType::Unstructured => {
                            VDMHeader::Unstructured(VDMHeaderUnstructured(raw.0))
                        }
                        VDMType::Structured => VDMHeader::Structured(VDMHeaderStructured(raw.0)),
                    }
                };

                trace!("VDM RX: {:?}", header);
                // trace!("HEADER: VDM:: TYPE: {:?}, VERS: {:?}", header.vdm_type(),
                // header.vdm_version()); trace!("HEADER: CMD:: TYPE: {:?}, CMD:
                // {:?}", header.command_type(), header.command());

                // Keep for now...
                // let pkt = payload
                //     .chunks_exact(1)
                //     .take(8)
                //     .map(|i| i[0])
                //     .collect::<Vec<u8, 8>>();

                Message::VendorDefined(header)
            }
            _ => {
                warn!("unknown message type");
                Message::Unknown
            }
        }
    }
}
