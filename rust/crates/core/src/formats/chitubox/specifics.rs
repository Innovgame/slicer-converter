/// Chitubox-family concrete profile kind.
///
/// Chitubox-family 的具体 profile 类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChituboxProfileKind {
    /// Generic CTB profile.
    ///
    /// 通用 CTB profile。
    Ctb,
    /// GKtwo CTB profile.
    ///
    /// GKtwo CTB profile。
    GktwoCtb,
    /// CBDDLP profile.
    ///
    /// CBDDLP profile。
    Cbddlp,
    /// Photon profile.
    ///
    /// Photon profile。
    Photon,
    /// PHZ profile.
    ///
    /// PHZ profile。
    Phz,
    /// FDG profile.
    ///
    /// FDG profile。
    Fdg,
    /// PhotonS profile.
    ///
    /// PhotonS profile。
    PhotonS,
    /// Encrypted CTB profile.
    ///
    /// encrypted CTB profile。
    EncryptedCtb,
    /// Chitubox ZIP/archive profile.
    ///
    /// Chitubox ZIP/archive profile。
    Zip,
}

/// Chitubox-family profile facts.
///
/// These facts describe the accepted magic/version matrix and optional CTB
/// layout details that catalog output and resolvers need alongside `FormatSpec`.
///
/// Chitubox-family profile facts。它描述可接受的 magic/version 矩阵，以及 catalog output
/// 和 resolver 在 `FormatSpec` 之外需要的可选 CTB layout details。
#[derive(Debug, Clone, Copy)]
pub struct ChituboxSpec {
    /// Concrete profile kind.
    ///
    /// 具体 profile 类型。
    pub kind: ChituboxProfileKind,
    /// CTB major version when this is a CTB-like profile.
    ///
    /// CTB-like profile 对应的 CTB major version。
    pub ctb_version: Option<u32>,
    /// Optional machine profile discriminator.
    ///
    /// 可选机器 profile 判别字段。
    pub machine_profile: Option<&'static str>,
    /// Accepted magic values.
    ///
    /// 可接受 magic values。
    pub allowed_magics: &'static [u32],
    /// Accepted file version values.
    ///
    /// 可接受 file version values。
    pub allowed_versions: &'static [u32],
    /// Version written when building this profile.
    ///
    /// 构建该 profile 时写入的 version。
    pub build_version: Option<u32>,
    /// Layer table entry size when fixed by the layout.
    ///
    /// layout 固定时的 layer table entry size。
    pub layer_table_entry_size: Option<u32>,
}
