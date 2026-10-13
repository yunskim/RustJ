//! Canonical IDs for the currently recognized primitive spellings.
//! IDs denote a J symbol, not a valence or a physical kernel.

/// Names from pinned jsource ws.c/t.c (13994ffa1ed5f06f79fad6e9822a7ed2d29b1528).
/// Naming metadata only: no numeric C opcode, executor, rank or admission proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveName {
    Eq,
    Asgn,
    Gasgn,
    Lt,
    Floor,
    Le,
    Gt,
    Ceil,
    Ge,
    Inf,
    Fcons,
    Plus,
    Plusdot,
    Plusco,
    Star,
    Stardot,
    Starco,
    Minus,
    Not,
    Match,
    Div,
    Domino,
    Root,
    Exp,
    Log,
    Powop,
    Dollar,
    Sparse,
    SelfReference,
    Tilde,
    Nub,
    Ne,
    Stile,
    Rev,
    Cant,
    Dot,
    Colon,
    Obverse,
    Adverse,
    Comma,
    Comdot,
    Lamin,
    Semico,
    Cut,
    Words,
    Pound,
    Base,
    Abase,
    At,
    Atdot,
    Atco,
    Slash,
    Sldot,
    Grade,
    Bslash,
    Bsdot,
    Dgrade,
    Left,
    Lev,
    Cap,
    Right,
    Dex,
    Ident,
    Lbrace,
    Take,
    Tail,
    Rbrace,
    Drop,
    Ctail,
    Grave,
    Grdot,
    Grco,
    Qq,
    Exec,
    Thorn,
    Amp,
    Under,
    Ampco,
    Bang,
    Fit,
    Ibeam,
    Query,
    Qrydot,
    Qryco,
    Lpar,
    Rpar,
    Alp,
    Ace,
    Atomic,
    Bdot,
    Cdot,
    Cycle,
    Eps,
    Ebar,
    Fix,
    Fixs,
    Hgeom,
    Iota,
    Ico,
    Icap,
    Jdot,
    Ldot,
    Lcapco,
    Mdot,
    Mcap,
    Circle,
    Poly,
    Pco,
    Qco,
    Rdot,
    Scapco,
    Tdot,
    Tcapdot,
    Udot,
    Uco,
    Vdot,
    Xco,
    Fdot,
    Fco,
    Zco,
    Fetch,
    Pderiv,
    Undco,
    Fdotdot,
    Fdotco,
    Fcodot,
    Fcoco,
    Sldotdot,
    Setself,
}

impl PrimitiveName {
    pub const fn jsource_name(self) -> &'static str {
        match self {
            Self::Eq => "CEQ",
            Self::Asgn => "CASGN",
            Self::Gasgn => "CGASGN",
            Self::Lt => "CLT",
            Self::Floor => "CFLOOR",
            Self::Le => "CLE",
            Self::Gt => "CGT",
            Self::Ceil => "CCEIL",
            Self::Ge => "CGE",
            Self::Inf => "CINF",
            Self::Fcons => "CFCONS",
            Self::Plus => "CPLUS",
            Self::Plusdot => "CPLUSDOT",
            Self::Plusco => "CPLUSCO",
            Self::Star => "CSTAR",
            Self::Stardot => "CSTARDOT",
            Self::Starco => "CSTARCO",
            Self::Minus => "CMINUS",
            Self::Not => "CNOT",
            Self::Match => "CMATCH",
            Self::Div => "CDIV",
            Self::Domino => "CDOMINO",
            Self::Root => "CROOT",
            Self::Exp => "CEXP",
            Self::Log => "CLOG",
            Self::Powop => "CPOWOP",
            Self::Dollar => "CDOLLAR",
            Self::Sparse => "CSPARSE",
            Self::SelfReference => "CSELF",
            Self::Tilde => "CTILDE",
            Self::Nub => "CNUB",
            Self::Ne => "CNE",
            Self::Stile => "CSTILE",
            Self::Rev => "CREV",
            Self::Cant => "CCANT",
            Self::Dot => "CDOT",
            Self::Colon => "CCOLON",
            Self::Obverse => "COBVERSE",
            Self::Adverse => "CADVERSE",
            Self::Comma => "CCOMMA",
            Self::Comdot => "CCOMDOT",
            Self::Lamin => "CLAMIN",
            Self::Semico => "CSEMICO",
            Self::Cut => "CCUT",
            Self::Words => "CWORDS",
            Self::Pound => "CPOUND",
            Self::Base => "CBASE",
            Self::Abase => "CABASE",
            Self::At => "CAT",
            Self::Atdot => "CATDOT",
            Self::Atco => "CATCO",
            Self::Slash => "CSLASH",
            Self::Sldot => "CSLDOT",
            Self::Grade => "CGRADE",
            Self::Bslash => "CBSLASH",
            Self::Bsdot => "CBSDOT",
            Self::Dgrade => "CDGRADE",
            Self::Left => "CLEFT",
            Self::Lev => "CLEV",
            Self::Cap => "CCAP",
            Self::Right => "CRIGHT",
            Self::Dex => "CDEX",
            Self::Ident => "CIDENT",
            Self::Lbrace => "CLBRACE",
            Self::Take => "CTAKE",
            Self::Tail => "CTAIL",
            Self::Rbrace => "CRBRACE",
            Self::Drop => "CDROP",
            Self::Ctail => "CCTAIL",
            Self::Grave => "CGRAVE",
            Self::Grdot => "CGRDOT",
            Self::Grco => "CGRCO",
            Self::Qq => "CQQ",
            Self::Exec => "CEXEC",
            Self::Thorn => "CTHORN",
            Self::Amp => "CAMP",
            Self::Under => "CUNDER",
            Self::Ampco => "CAMPCO",
            Self::Bang => "CBANG",
            Self::Fit => "CFIT",
            Self::Ibeam => "CIBEAM",
            Self::Query => "CQUERY",
            Self::Qrydot => "CQRYDOT",
            Self::Qryco => "CQRYCO",
            Self::Lpar => "CLPAR",
            Self::Rpar => "CRPAR",
            Self::Alp => "CALP",
            Self::Ace => "CACE",
            Self::Atomic => "CATOMIC",
            Self::Bdot => "CBDOT",
            Self::Cdot => "CCDOT",
            Self::Cycle => "CCYCLE",
            Self::Eps => "CEPS",
            Self::Ebar => "CEBAR",
            Self::Fix => "CFIX",
            Self::Fixs => "CFIXS",
            Self::Hgeom => "CHGEOM",
            Self::Iota => "CIOTA",
            Self::Ico => "CICO",
            Self::Icap => "CICAP",
            Self::Jdot => "CJDOT",
            Self::Ldot => "CLDOT",
            Self::Lcapco => "CLCAPCO",
            Self::Mdot => "CMDOT",
            Self::Mcap => "CMCAP",
            Self::Circle => "CCIRCLE",
            Self::Poly => "CPOLY",
            Self::Pco => "CPCO",
            Self::Qco => "CQCO",
            Self::Rdot => "CRDOT",
            Self::Scapco => "CSCAPCO",
            Self::Tdot => "CTDOT",
            Self::Tcapdot => "CTCAPDOT",
            Self::Udot => "CUDOT",
            Self::Uco => "CUCO",
            Self::Vdot => "CVDOT",
            Self::Xco => "CXCO",
            Self::Fdot => "CFDOT",
            Self::Fco => "CFCO",
            Self::Zco => "CZCO",
            Self::Fetch => "CFETCH",
            Self::Pderiv => "CPDERIV",
            Self::Undco => "CUNDCO",
            Self::Fdotdot => "CFDOTDOT",
            Self::Fdotco => "CFDOTCO",
            Self::Fcodot => "CFCODOT",
            Self::Fcoco => "CFCOCO",
            Self::Sldotdot => "CSLDOTDOT",
            Self::Setself => "CSETSELF",
        }
    }
}

/// Source handler names are provenance, not Rust execution implementations.
/// Atomic handlers may serve several meanings; no semantic contract is inferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimitiveNames {
    pub spelling: &'static str,
    pub name: PrimitiveName,
    pub monad: Option<&'static str>,
    pub dyad: Option<&'static str>,
}
impl PrimitiveNames {
    /// Public ws.c spellings, including nouns and syntax tokens.
    /// Those entries do not become callable functions through this table.
    pub const ALL: &'static [Self] = &[
        Self {
            spelling: "=",
            name: PrimitiveName::Eq,
            monad: Some("jtsclass"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "=.",
            name: PrimitiveName::Asgn,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "=:",
            name: PrimitiveName::Gasgn,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "<",
            name: PrimitiveName::Lt,
            monad: Some("jtbox"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "<.",
            name: PrimitiveName::Floor,
            monad: Some("jtatomic1"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "<:",
            name: PrimitiveName::Le,
            monad: Some("jtdecrem"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: ">",
            name: PrimitiveName::Gt,
            monad: Some("jtope"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: ">.",
            name: PrimitiveName::Ceil,
            monad: Some("jtatomic1"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: ">:",
            name: PrimitiveName::Ge,
            monad: Some("jtincrem"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "_",
            name: PrimitiveName::Inf,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "_:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "+",
            name: PrimitiveName::Plus,
            monad: Some("jtatomic1"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "+.",
            name: PrimitiveName::Plusdot,
            monad: Some("jtrect"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "+:",
            name: PrimitiveName::Plusco,
            monad: Some("jtduble"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "*",
            name: PrimitiveName::Star,
            monad: Some("jtatomic1"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "*.",
            name: PrimitiveName::Stardot,
            monad: Some("jtpolar"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "*:",
            name: PrimitiveName::Starco,
            monad: Some("jtsquare"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "-",
            name: PrimitiveName::Minus,
            monad: Some("jtnegate"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "-.",
            name: PrimitiveName::Not,
            monad: Some("jtnot"),
            dyad: Some("jtless"),
        },
        Self {
            spelling: "-:",
            name: PrimitiveName::Match,
            monad: Some("jthalve"),
            dyad: Some("jtmatch"),
        },
        Self {
            spelling: "%",
            name: PrimitiveName::Div,
            monad: Some("jtrecip"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "%.",
            name: PrimitiveName::Domino,
            monad: Some("jtminv"),
            dyad: Some("jtmdiv"),
        },
        Self {
            spelling: "%:",
            name: PrimitiveName::Root,
            monad: Some("jtatomic1"),
            dyad: Some("jtroot"),
        },
        Self {
            spelling: "^",
            name: PrimitiveName::Exp,
            monad: Some("jtatomic1"),
            dyad: Some("jtexpn2"),
        },
        Self {
            spelling: "^.",
            name: PrimitiveName::Log,
            monad: Some("jtatomic1"),
            dyad: Some("jtlogar2"),
        },
        Self {
            spelling: "^:",
            name: PrimitiveName::Powop,
            monad: None,
            dyad: Some("jtpowop"),
        },
        Self {
            spelling: "$",
            name: PrimitiveName::Dollar,
            monad: Some("jtshapex"),
            dyad: Some("jtreitem"),
        },
        Self {
            spelling: "$.",
            name: PrimitiveName::Sparse,
            monad: Some("jtsparse1"),
            dyad: Some("jtsparse2"),
        },
        Self {
            spelling: "$:",
            name: PrimitiveName::SelfReference,
            monad: Some("jtself12"),
            dyad: Some("jtself12"),
        },
        Self {
            spelling: "~",
            name: PrimitiveName::Tilde,
            monad: Some("jtswap"),
            dyad: None,
        },
        Self {
            spelling: "~.",
            name: PrimitiveName::Nub,
            monad: Some("jtnub"),
            dyad: None,
        },
        Self {
            spelling: "~:",
            name: PrimitiveName::Ne,
            monad: Some("jtnubsieve"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "|",
            name: PrimitiveName::Stile,
            monad: Some("jtatomic1"),
            dyad: Some("jtresidue"),
        },
        Self {
            spelling: "|.",
            name: PrimitiveName::Rev,
            monad: Some("jtreverse"),
            dyad: Some("jtrotate"),
        },
        Self {
            spelling: "|:",
            name: PrimitiveName::Cant,
            monad: Some("jtcant1"),
            dyad: Some("jtcant2"),
        },
        Self {
            spelling: ".",
            name: PrimitiveName::Dot,
            monad: None,
            dyad: Some("jtdot"),
        },
        Self {
            spelling: ":",
            name: PrimitiveName::Colon,
            monad: None,
            dyad: Some("jtcolon"),
        },
        Self {
            spelling: ":.",
            name: PrimitiveName::Obverse,
            monad: None,
            dyad: Some("jtobverse"),
        },
        Self {
            spelling: "::",
            name: PrimitiveName::Adverse,
            monad: None,
            dyad: Some("jtadverse"),
        },
        Self {
            spelling: ",",
            name: PrimitiveName::Comma,
            monad: Some("jtravel"),
            dyad: Some("jtapip"),
        },
        Self {
            spelling: ",.",
            name: PrimitiveName::Comdot,
            monad: Some("jttable"),
            dyad: Some("jtstitch"),
        },
        Self {
            spelling: ",:",
            name: PrimitiveName::Lamin,
            monad: Some("jtlamin1"),
            dyad: Some("jtlamin2"),
        },
        Self {
            spelling: ";",
            name: PrimitiveName::Semico,
            monad: Some("jtraze"),
            dyad: Some("jtjlink"),
        },
        Self {
            spelling: ";.",
            name: PrimitiveName::Cut,
            monad: None,
            dyad: Some("jtcut"),
        },
        Self {
            spelling: ";:",
            name: PrimitiveName::Words,
            monad: Some("jtwords"),
            dyad: Some("jtfsm"),
        },
        Self {
            spelling: "#",
            name: PrimitiveName::Pound,
            monad: Some("jttally"),
            dyad: Some("jtrepeat"),
        },
        Self {
            spelling: "#.",
            name: PrimitiveName::Base,
            monad: Some("jtbase1"),
            dyad: Some("jtbase2"),
        },
        Self {
            spelling: "#:",
            name: PrimitiveName::Abase,
            monad: Some("jtabase1"),
            dyad: Some("jtabase2"),
        },
        Self {
            spelling: "@",
            name: PrimitiveName::At,
            monad: None,
            dyad: Some("jtatop"),
        },
        Self {
            spelling: "@.",
            name: PrimitiveName::Atdot,
            monad: None,
            dyad: Some("jtagendai"),
        },
        Self {
            spelling: "@:",
            name: PrimitiveName::Atco,
            monad: None,
            dyad: Some("jtatco"),
        },
        Self {
            spelling: "/",
            name: PrimitiveName::Slash,
            monad: Some("jtslash"),
            dyad: None,
        },
        Self {
            spelling: "/.",
            name: PrimitiveName::Sldot,
            monad: Some("jtsldot"),
            dyad: None,
        },
        Self {
            spelling: "/:",
            name: PrimitiveName::Grade,
            monad: Some("jtgrade1"),
            dyad: Some("jtgrade2"),
        },
        Self {
            spelling: "\\",
            name: PrimitiveName::Bslash,
            monad: Some("jtbslash"),
            dyad: None,
        },
        Self {
            spelling: "\\.",
            name: PrimitiveName::Bsdot,
            monad: Some("jtbsdot"),
            dyad: None,
        },
        Self {
            spelling: "\\:",
            name: PrimitiveName::Dgrade,
            monad: Some("jtdgrade1"),
            dyad: Some("jtdgrade2"),
        },
        Self {
            spelling: "[",
            name: PrimitiveName::Left,
            monad: Some("jtright1"),
            dyad: Some("jtleft2"),
        },
        Self {
            spelling: "[.",
            name: PrimitiveName::Lev,
            monad: None,
            dyad: Some("jtlev"),
        },
        Self {
            spelling: "[:",
            name: PrimitiveName::Cap,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "]",
            name: PrimitiveName::Right,
            monad: Some("jtright1"),
            dyad: Some("jtright2"),
        },
        Self {
            spelling: "].",
            name: PrimitiveName::Dex,
            monad: Some("jtlev"),
            dyad: None,
        },
        Self {
            spelling: "]:",
            name: PrimitiveName::Ident,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "{",
            name: PrimitiveName::Lbrace,
            monad: Some("jtcatalog"),
            dyad: Some("jtfrom"),
        },
        Self {
            spelling: "{.",
            name: PrimitiveName::Take,
            monad: Some("jthead"),
            dyad: Some("jttake"),
        },
        Self {
            spelling: "{:",
            name: PrimitiveName::Tail,
            monad: Some("jttail"),
            dyad: None,
        },
        Self {
            spelling: "}",
            name: PrimitiveName::Rbrace,
            monad: Some("jtamend"),
            dyad: None,
        },
        Self {
            spelling: "}.",
            name: PrimitiveName::Drop,
            monad: Some("jtbehead"),
            dyad: Some("jtdrop"),
        },
        Self {
            spelling: "}:",
            name: PrimitiveName::Ctail,
            monad: Some("jtcurtail"),
            dyad: None,
        },
        Self {
            spelling: "`",
            name: PrimitiveName::Grave,
            monad: None,
            dyad: Some("jttie"),
        },
        Self {
            spelling: "`.",
            name: PrimitiveName::Grdot,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "`:",
            name: PrimitiveName::Grco,
            monad: None,
            dyad: Some("jtevger"),
        },
        Self {
            spelling: "\"",
            name: PrimitiveName::Qq,
            monad: None,
            dyad: Some("jtqq"),
        },
        Self {
            spelling: "\".",
            name: PrimitiveName::Exec,
            monad: Some("jtexec1"),
            dyad: Some("jtexec2"),
        },
        Self {
            spelling: "\":",
            name: PrimitiveName::Thorn,
            monad: Some("jtthorn1"),
            dyad: Some("jtthorn2"),
        },
        Self {
            spelling: "&",
            name: PrimitiveName::Amp,
            monad: None,
            dyad: Some("jtamp"),
        },
        Self {
            spelling: "&.",
            name: PrimitiveName::Under,
            monad: None,
            dyad: Some("jtunder"),
        },
        Self {
            spelling: "&:",
            name: PrimitiveName::Ampco,
            monad: None,
            dyad: Some("jtampco"),
        },
        Self {
            spelling: "!",
            name: PrimitiveName::Bang,
            monad: Some("jtatomic1"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "!.",
            name: PrimitiveName::Fit,
            monad: None,
            dyad: Some("jtfit"),
        },
        Self {
            spelling: "!:",
            name: PrimitiveName::Ibeam,
            monad: None,
            dyad: Some("jtforeign"),
        },
        Self {
            spelling: "?",
            name: PrimitiveName::Query,
            monad: Some("jtroll"),
            dyad: Some("jtdeal"),
        },
        Self {
            spelling: "?.",
            name: PrimitiveName::Qrydot,
            monad: Some("jtrollx"),
            dyad: Some("jtdealx"),
        },
        Self {
            spelling: "?:",
            name: PrimitiveName::Qryco,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "(",
            name: PrimitiveName::Lpar,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: ")",
            name: PrimitiveName::Rpar,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "a.",
            name: PrimitiveName::Alp,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "a:",
            name: PrimitiveName::Ace,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "A.",
            name: PrimitiveName::Atomic,
            monad: Some("jtadot1"),
            dyad: Some("jtadot2"),
        },
        Self {
            spelling: "b.",
            name: PrimitiveName::Bdot,
            monad: Some("jtbdot"),
            dyad: None,
        },
        Self {
            spelling: "c.",
            name: PrimitiveName::Cdot,
            monad: None,
            dyad: Some("jtcdot2"),
        },
        Self {
            spelling: "C.",
            name: PrimitiveName::Cycle,
            monad: Some("jtcdot1"),
            dyad: Some("jtccapdot2"),
        },
        Self {
            spelling: "e.",
            name: PrimitiveName::Eps,
            monad: Some("jtrazein"),
            dyad: Some("jteps"),
        },
        Self {
            spelling: "E.",
            name: PrimitiveName::Ebar,
            monad: None,
            dyad: Some("jtebar"),
        },
        Self {
            spelling: "f.",
            name: PrimitiveName::Fix,
            monad: Some("jtfix"),
            dyad: None,
        },
        Self {
            spelling: "f:",
            name: PrimitiveName::Fixs,
            monad: None,
            dyad: Some("jtfix"),
        },
        Self {
            spelling: "H.",
            name: PrimitiveName::Hgeom,
            monad: None,
            dyad: Some("jthgeom"),
        },
        Self {
            spelling: "i.",
            name: PrimitiveName::Iota,
            monad: Some("jtiota"),
            dyad: Some("jtindexof"),
        },
        Self {
            spelling: "i:",
            name: PrimitiveName::Ico,
            monad: Some("jtjico1"),
            dyad: Some("jtjico2"),
        },
        Self {
            spelling: "I.",
            name: PrimitiveName::Icap,
            monad: Some("jticap"),
            dyad: Some("jticap2"),
        },
        Self {
            spelling: "j.",
            name: PrimitiveName::Jdot,
            monad: Some("jtjdot1"),
            dyad: Some("jtjdot2"),
        },
        Self {
            spelling: "L.",
            name: PrimitiveName::Ldot,
            monad: Some("jtlevel1"),
            dyad: None,
        },
        Self {
            spelling: "L:",
            name: PrimitiveName::Lcapco,
            monad: None,
            dyad: Some("jtlcapco"),
        },
        Self {
            spelling: "m.",
            name: PrimitiveName::Mdot,
            monad: None,
            dyad: Some("jtmdot"),
        },
        Self {
            spelling: "M.",
            name: PrimitiveName::Mcap,
            monad: Some("jtmemo"),
            dyad: None,
        },
        Self {
            spelling: "o.",
            name: PrimitiveName::Circle,
            monad: Some("jtpix"),
            dyad: Some("jtatomic2"),
        },
        Self {
            spelling: "p.",
            name: PrimitiveName::Poly,
            monad: Some("jtpoly1"),
            dyad: Some("jtpoly2"),
        },
        Self {
            spelling: "p:",
            name: PrimitiveName::Pco,
            monad: Some("jtprime"),
            dyad: Some("jtpco2"),
        },
        Self {
            spelling: "q:",
            name: PrimitiveName::Qco,
            monad: Some("jtfactor"),
            dyad: Some("jtqco2"),
        },
        Self {
            spelling: "r.",
            name: PrimitiveName::Rdot,
            monad: Some("jtrdot1"),
            dyad: Some("jtrdot2"),
        },
        Self {
            spelling: "S:",
            name: PrimitiveName::Scapco,
            monad: None,
            dyad: Some("jtscapco"),
        },
        Self {
            spelling: "t.",
            name: PrimitiveName::Tdot,
            monad: None,
            dyad: Some("jttdot"),
        },
        Self {
            spelling: "T.",
            name: PrimitiveName::Tcapdot,
            monad: Some("jttcapdot1"),
            dyad: Some("jttcapdot2"),
        },
        Self {
            spelling: "u.",
            name: PrimitiveName::Udot,
            monad: Some("jtimplocref"),
            dyad: Some("jtimplocref"),
        },
        Self {
            spelling: "u:",
            name: PrimitiveName::Uco,
            monad: Some("jtuco1"),
            dyad: Some("jtuco2"),
        },
        Self {
            spelling: "v.",
            name: PrimitiveName::Vdot,
            monad: Some("jtimplocref"),
            dyad: Some("jtimplocref"),
        },
        Self {
            spelling: "x:",
            name: PrimitiveName::Xco,
            monad: Some("jtxco1"),
            dyad: Some("jtxco2"),
        },
        Self {
            spelling: "0:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "1:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "2:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "3:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "4:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "5:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "6:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "7:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "8:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "9:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "F.",
            name: PrimitiveName::Fdot,
            monad: None,
            dyad: Some("jtfold"),
        },
        Self {
            spelling: "F:",
            name: PrimitiveName::Fco,
            monad: None,
            dyad: Some("jtfold"),
        },
        Self {
            spelling: "Z:",
            name: PrimitiveName::Zco,
            monad: Some("jtfoldZ1"),
            dyad: Some("jtfoldZ2"),
        },
        Self {
            spelling: "{::",
            name: PrimitiveName::Fetch,
            monad: Some("jtmap"),
            dyad: Some("jtfetch"),
        },
        Self {
            spelling: "p..",
            name: PrimitiveName::Pderiv,
            monad: Some("jtpderiv1"),
            dyad: Some("jtpderiv2"),
        },
        Self {
            spelling: "&.:",
            name: PrimitiveName::Undco,
            monad: None,
            dyad: Some("jtundco"),
        },
        Self {
            spelling: "F..",
            name: PrimitiveName::Fdotdot,
            monad: None,
            dyad: Some("jtfold"),
        },
        Self {
            spelling: "F.:",
            name: PrimitiveName::Fdotco,
            monad: None,
            dyad: Some("jtfold"),
        },
        Self {
            spelling: "F:.",
            name: PrimitiveName::Fcodot,
            monad: None,
            dyad: Some("jtfold"),
        },
        Self {
            spelling: "F::",
            name: PrimitiveName::Fcoco,
            monad: None,
            dyad: Some("jtfold"),
        },
        Self {
            spelling: "/..",
            name: PrimitiveName::Sldotdot,
            monad: Some("jtsldotdot"),
            dyad: None,
        },
        Self {
            spelling: "$::",
            name: PrimitiveName::Setself,
            monad: Some("jtdolcoco"),
            dyad: None,
        },
        Self {
            spelling: "_1:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "_2:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "_3:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "_4:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "_5:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "_6:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "_7:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "_8:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "_9:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
        Self {
            spelling: "__:",
            name: PrimitiveName::Fcons,
            monad: None,
            dyad: None,
        },
    ];
    pub fn from_spelling(spelling: &str) -> Option<&'static Self> {
        Self::ALL.iter().find(|entry| entry.spelling == spelling)
    }
}

pub const REGISTRY_VERSION: u32 = 11;
macro_rules! primitives {
    ($($id:ident => $spelling:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum PrimitiveId { $($id),+ }
        impl PrimitiveId {
    pub fn names(self) -> &'static PrimitiveNames {
        PrimitiveNames::from_spelling(self.spelling()).expect("named core primitive")
    }

            pub const ALL: &'static [Self] = &[$(Self::$id),+];
            pub fn from_spelling(s: &str) -> Option<Self> {
                match s { $($spelling => Some(Self::$id),)+ _ => None }
            }
            pub const fn spelling(self) -> &'static str {
                match self { $(Self::$id => $spelling),+ }
            }
        }
    }
}
primitives! {
    Plus => "+",
    Minus => "-",
    Star => "*",
    Div => "%",
    Dollar => "$",
    Sparse => "$.",
    Pound => "#",
    Comma => ",",
    Eq => "=",
    Lt => "<",
    Gt => ">",
    Lbrace => "{",
    Stile => "|",
    Iota => "i.",
    Rev => "|.",
    Cant => "|:",
    Take => "{.",
    Drop => "}.",
    Ico => "i:",
    Icap => "I.",
    Eps => "e.",
    Ebar => "E.",
    Udot => "u.",
    Vdot => "v.",
    Cap => "[:",
    Left => "[",
    Right => "]",
    Bang => "!",
    Exec => "\".",
    Thorn => "\":",
    Base => "#.",
    Abase => "#:",
    SelfReference => "$:",
    Domino => "%.",
    Root => "%:",
    Stardot => "*.",
    Starco => "*:",
    Plusdot => "+.",
    Plusco => "+:",
    Comdot => ",.",
    Lamin => ",:",
    Not => "-.",
    Match => "-:",
    Grade => "/:",
    Fcons0 => "0:",
    Fcons1 => "1:",
    Fcons2 => "2:",
    Fcons3 => "3:",
    Fcons4 => "4:",
    Fcons5 => "5:",
    Fcons6 => "6:",
    Fcons7 => "7:",
    Fcons8 => "8:",
    Fcons9 => "9:",
    Semico => ";",
    Words => ";:",
    Floor => "<.",
    Le => "<:",
    Ceil => ">.",
    Ge => ">:",
    Query => "?",
    Qrydot => "?.",
    Atomic => "A.",
    Cycle => "C.",
    Ldot => "L.",
    Tcapdot => "T.",
    Zco => "Z:",
    Dgrade => "\\:",
    Exp => "^",
    Log => "^.",
    FconsNeg1 => "_1:",
    FconsNeg2 => "_2:",
    FconsNeg3 => "_3:",
    FconsNeg4 => "_4:",
    FconsNeg5 => "_5:",
    FconsNeg6 => "_6:",
    FconsNeg7 => "_7:",
    FconsNeg8 => "_8:",
    FconsNeg9 => "_9:",
    FconsInf => "_:",
    FconsNegInf => "__:",
    Cdot => "c.",
    Jdot => "j.",
    Circle => "o.",
    Poly => "p.",
    Pderiv => "p..",
    Pco => "p:",
    Qco => "q:",
    Rdot => "r.",
    Uco => "u:",
    Xco => "x:",
    Tail => "{:",
    Fetch => "{::",
    Ctail => "}:",
    Nub => "~.",
    Ne => "~:",
}

impl PrimitiveId {
    pub const fn support(self) -> PrimitiveSupport {
        use PrimitiveId::*;
        match self {
            Plus | Minus | Star | Div | Dollar | Sparse | Pound | Comma | Eq | Lt | Gt | Lbrace
            | Stile | Iota | Rev | Cant | Take | Drop | Ico | Icap | Eps | Ebar | Udot | Vdot
            | Cap | Left | Right => PrimitiveSupport::Bounded,
            _ => PrimitiveSupport::Unimplemented,
        }
    }
    /// C t.c's intrinsic (monad, dyad-left, dyad-right) header ranks.
    /// jsource RMAX is 63: six rank bits in RANKT, leaving two flag bits.
    /// It is also the maximum permitted array rank; an actual rank-63
    /// argument is a 63-dimensional array. As an innate verb rank, 63
    /// acts as infinite rank over permitted arrays: the effective cell
    /// rank is capped by the argument rank, so the whole argument is a cell.
    /// Retain this integer representation for current jsource compatibility.
    pub const fn innate_ranks(self) -> Option<[i64; 3]> {
        use PrimitiveId::*;
        match self {
            Plus | Minus | Star | Div | Gt | Stile => Some([0, 0, 0]),
            Eq | Lt => Some([63, 0, 0]),
            Dollar | Pound | Rev | Cant | Take | Drop => Some([63, 1, 63]),
            Lbrace => Some([1, 0, 63]),
            Iota | Icap => Some([1, 63, 63]),
            Ico | Ebar => Some([0, 63, 63]),
            Sparse | Comma | Eps | Udot | Vdot | Cap | Left | Right => Some([63; 3]),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdverbId {
    Slash,
    /// J `/.`: a verb-derived noun group/oblique function. Source identity
    /// is constructed independently of its monadic/dyadic runtime semantics.
    Sldot,
    /// J prefix/infix adverb `\`. The derived verb keeps the adverb
    /// identity; monadic prefix vs dyadic infix is resolved at application.
    Bslash,
    /// Ident `]:` returns its noun or verb operand without invoking it.
    Ident,
    Setself,
    Sldotdot,
    Mcap,
    Bsdot,
    Bdot,
    Fix,
    Rbrace,
    Tilde,
}
impl AdverbId {
    pub const ALL: &'static [Self] = &[
        Self::Slash,
        Self::Sldot,
        Self::Bslash,
        Self::Ident,
        Self::Setself,
        Self::Sldotdot,
        Self::Mcap,
        Self::Bsdot,
        Self::Bdot,
        Self::Fix,
        Self::Rbrace,
        Self::Tilde,
    ];
    pub const fn support(self) -> PrimitiveSupport {
        match self {
            Self::Slash | Self::Sldot | Self::Bslash | Self::Ident => PrimitiveSupport::Bounded,
            _ => PrimitiveSupport::Unimplemented,
        }
    }
    pub fn names(self) -> &'static PrimitiveNames {
        PrimitiveNames::from_spelling(self.spelling()).expect("named core primitive")
    }

    pub fn from_spelling(s: &str) -> Option<Self> {
        match s {
            "/" => Some(Self::Slash),
            "/." => Some(Self::Sldot),
            "\\" => Some(Self::Bslash),
            "]:" => Some(Self::Ident),
            "$::" => Some(Self::Setself),
            "/.." => Some(Self::Sldotdot),
            "M." => Some(Self::Mcap),
            "\\." => Some(Self::Bsdot),
            "b." => Some(Self::Bdot),
            "f." => Some(Self::Fix),
            "}" => Some(Self::Rbrace),
            "~" => Some(Self::Tilde),
            _ => None,
        }
    }
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Slash => "/",
            Self::Sldot => "/.",
            Self::Bslash => "\\",
            Self::Ident => "]:",
            Self::Setself => "$::",
            Self::Sldotdot => "/..",
            Self::Mcap => "M.",
            Self::Bsdot => "\\.",
            Self::Bdot => "b.",
            Self::Fix => "f.",
            Self::Rbrace => "}",
            Self::Tilde => "~",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConjunctionId {
    Qq,
    /// Historical internal name: spelling `@:` is NuVoc At, not rank-sensitive `@` Atco.
    Atco,
    Lev,
    Dex,
    Fit,
    Ibeam,
    Amp,
    Under,
    Undco,
    Ampco,
    Dot,
    Colon,
    Obverse,
    Adverse,
    Cut,
    At,
    Atdot,
    Fdot,
    Fdotdot,
    Fdotco,
    Fco,
    Fcodot,
    Fcoco,
    Hgeom,
    Lcapco,
    Scapco,
    Powop,
    Grave,
    Grco,
    Fixs,
    Mdot,
    Tdot,
}
impl ConjunctionId {
    pub const ALL: &'static [Self] = &[
        Self::Qq,
        Self::Atco,
        Self::Lev,
        Self::Dex,
        Self::Fit,
        Self::Ibeam,
        Self::Amp,
        Self::Under,
        Self::Undco,
        Self::Ampco,
        Self::Dot,
        Self::Colon,
        Self::Obverse,
        Self::Adverse,
        Self::Cut,
        Self::At,
        Self::Atdot,
        Self::Fdot,
        Self::Fdotdot,
        Self::Fdotco,
        Self::Fco,
        Self::Fcodot,
        Self::Fcoco,
        Self::Hgeom,
        Self::Lcapco,
        Self::Scapco,
        Self::Powop,
        Self::Grave,
        Self::Grco,
        Self::Fixs,
        Self::Mdot,
        Self::Tdot,
    ];
    pub const fn support(self) -> PrimitiveSupport {
        match self {
            Self::Qq | Self::Atco | Self::Lev | Self::Dex => PrimitiveSupport::Bounded,
            _ => PrimitiveSupport::Unimplemented,
        }
    }
    pub fn names(self) -> &'static PrimitiveNames {
        PrimitiveNames::from_spelling(self.spelling()).expect("named core primitive")
    }

    pub fn from_spelling(s: &str) -> Option<Self> {
        match s {
            "\"" => Some(Self::Qq),
            "@:" => Some(Self::Atco),
            "[." => Some(Self::Lev),
            "]." => Some(Self::Dex),
            "!." => Some(Self::Fit),
            "!:" => Some(Self::Ibeam),
            "&" => Some(Self::Amp),
            "&." => Some(Self::Under),
            "&.:" => Some(Self::Undco),
            "&:" => Some(Self::Ampco),
            "." => Some(Self::Dot),
            ":" => Some(Self::Colon),
            ":." => Some(Self::Obverse),
            "::" => Some(Self::Adverse),
            ";." => Some(Self::Cut),
            "@" => Some(Self::At),
            "@." => Some(Self::Atdot),
            "F." => Some(Self::Fdot),
            "F.." => Some(Self::Fdotdot),
            "F.:" => Some(Self::Fdotco),
            "F:" => Some(Self::Fco),
            "F:." => Some(Self::Fcodot),
            "F::" => Some(Self::Fcoco),
            "H." => Some(Self::Hgeom),
            "L:" => Some(Self::Lcapco),
            "S:" => Some(Self::Scapco),
            "^:" => Some(Self::Powop),
            "`" => Some(Self::Grave),
            "`:" => Some(Self::Grco),
            "f:" => Some(Self::Fixs),
            "m." => Some(Self::Mdot),
            "t." => Some(Self::Tdot),
            _ => None,
        }
    }
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Qq => "\"",
            Self::Atco => "@:",
            Self::Lev => "[.",
            Self::Dex => "].",
            Self::Fit => "!.",
            Self::Ibeam => "!:",
            Self::Amp => "&",
            Self::Under => "&.",
            Self::Undco => "&.:",
            Self::Ampco => "&:",
            Self::Dot => ".",
            Self::Colon => ":",
            Self::Obverse => ":.",
            Self::Adverse => "::",
            Self::Cut => ";.",
            Self::At => "@",
            Self::Atdot => "@.",
            Self::Fdot => "F.",
            Self::Fdotdot => "F..",
            Self::Fdotco => "F.:",
            Self::Fco => "F:",
            Self::Fcodot => "F:.",
            Self::Fcoco => "F::",
            Self::Hgeom => "H.",
            Self::Lcapco => "L:",
            Self::Scapco => "S:",
            Self::Powop => "^:",
            Self::Grave => "`",
            Self::Grco => "`:",
            Self::Fixs => "f:",
            Self::Mdot => "m.",
            Self::Tdot => "t.",
        }
    }
}

/// Implementation status is metadata, never a distinct core identity.
/// Bounded implementations still require valence/type/rank/route verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveSupport {
    Bounded,
    Unimplemented,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveSemanticId {
    Verb(PrimitiveId),
    Adverb(AdverbId),
    Conjunction(ConjunctionId),
    Extension(&'static str),
}

impl PrimitiveSemanticId {
    pub const fn support(self) -> Option<PrimitiveSupport> {
        match self {
            Self::Verb(id) => Some(id.support()),
            Self::Adverb(id) => Some(id.support()),
            Self::Conjunction(id) => Some(id.support()),
            Self::Extension(_) => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitivePartOfSpeech {
    Verb,
    Adverb,
    Conjunction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveSourceOrigin {
    Core,
    Extension,
}

/// Target-independent lowering identity.  Backends resolve this key only after
/// semantic parsing/lowering; the enqueuer never selects a hardware route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoweringKey {
    Core(PrimitiveSemanticId),
    Extension(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrimitiveSemanticInfo {
    pub registry_version: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrimitiveHandle {
    pub semantic_id: PrimitiveSemanticId,
    pub source_origin: PrimitiveSourceOrigin,
    pub result_pos: PrimitivePartOfSpeech,
    pub semantic_info: PrimitiveSemanticInfo,
    pub lowering_key: LoweringKey,
}

impl PrimitiveHandle {
    fn core(semantic_id: PrimitiveSemanticId) -> Self {
        let result_pos = match semantic_id {
            PrimitiveSemanticId::Verb(_) => PrimitivePartOfSpeech::Verb,
            PrimitiveSemanticId::Adverb(_) => PrimitivePartOfSpeech::Adverb,
            PrimitiveSemanticId::Conjunction(_) => PrimitivePartOfSpeech::Conjunction,
            PrimitiveSemanticId::Extension(_) => {
                unreachable!("extension handles must declare their parser part of speech")
            }
        };
        Self {
            semantic_id,
            source_origin: PrimitiveSourceOrigin::Core,
            result_pos,
            semantic_info: PrimitiveSemanticInfo {
                registry_version: REGISTRY_VERSION,
            },
            lowering_key: LoweringKey::Core(semantic_id),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ExtensionPrimitive {
    pub spelling: &'static str,
    pub handle: PrimitiveHandle,
}

/// Semantic primitive lookup fixed for one compilation profile.
///
/// Core J spellings always win. Enabled extensions participate through the same
/// handle interface but cannot shadow core J semantics.
#[derive(Clone, Debug, Default)]
pub struct PrimitiveResolver {
    extensions: Vec<ExtensionPrimitive>,
}

impl PrimitiveResolver {
    pub fn core() -> Self {
        Self::default()
    }

    /// Admit only consistent extension identities at a compile-profile boundary.
    ///
    /// A descriptor is metadata, not proof of executable semantics. Even an
    /// admitted name stays a normal J NAME until parser-time binding lookup.
    pub fn with_extensions(
        extensions: impl IntoIterator<Item = ExtensionPrimitive>,
    ) -> Result<Self, String> {
        let mut admitted: Vec<ExtensionPrimitive> = Vec::new();
        for extension in extensions {
            let spelling = extension.spelling;
            // Consult the existing J word classifier, not a second spelling
            // grammar. Explicit locatives are not registry-owned base names.
            let words = crate::enqueuer::enqueue(spelling)
                .map_err(|_| format!("invalid extension NAME: {spelling}"))?;
            if !matches!(words.as_slice(), [word]
                if word.class == crate::enqueuer::EnqueueClass::Name
                    && word.span == (0..spelling.len())
                    && !word.flags.name_form.is_locative())
            {
                return Err(format!(
                    "extension must be a single ordinary J NAME: {spelling}"
                ));
            }
            if admitted
                .iter()
                .any(|previous| previous.spelling == spelling)
            {
                return Err(format!("duplicate extension NAME: {spelling}"));
            }
            let handle = extension.handle;
            let PrimitiveSemanticId::Extension(identity) = handle.semantic_id else {
                return Err(format!(
                    "extension has non-extension semantic identity: {spelling}"
                ));
            };
            if handle.source_origin != PrimitiveSourceOrigin::Extension
                || handle.lowering_key != LoweringKey::Extension(identity)
                || handle.semantic_info.registry_version != REGISTRY_VERSION
            {
                return Err(format!("inconsistent extension handle: {spelling}"));
            }
            admitted.push(extension);
        }
        Ok(Self {
            extensions: admitted,
        })
    }

    /// Resolve only spellings that are J core primitives at enqueue time.
    ///
    /// Alphabetic extension names must remain ordinary NAMEs. Their current
    /// binding and part of speech are resolved by the parser/name environment.
    pub fn resolve_core_for_enqueue(&self, spelling: &str) -> Option<PrimitiveHandle> {
        if let Some(id) = PrimitiveId::from_spelling(spelling) {
            return Some(PrimitiveHandle::core(PrimitiveSemanticId::Verb(id)));
        }
        if let Some(id) = AdverbId::from_spelling(spelling) {
            return Some(PrimitiveHandle::core(PrimitiveSemanticId::Adverb(id)));
        }
        if let Some(id) = ConjunctionId::from_spelling(spelling) {
            return Some(PrimitiveHandle::core(PrimitiveSemanticId::Conjunction(id)));
        }
        None
    }

    /// Lookup for the parser/name-binding layer after a word has entered as NAME.
    pub fn resolve_extension_binding(&self, name: &str) -> Option<PrimitiveHandle> {
        self.extensions
            .iter()
            .find(|extension| extension.spelling == name)
            .map(|extension| extension.handle)
    }
}

#[derive(Clone, Debug)]
pub struct PrimitiveContext {
    resolver: PrimitiveResolver,
}

impl PrimitiveContext {
    pub fn core() -> Self {
        Self {
            resolver: PrimitiveResolver::core(),
        }
    }

    pub fn new(resolver: PrimitiveResolver) -> Self {
        Self { resolver }
    }

    pub fn resolve_core_for_enqueue(&self, spelling: &str) -> Option<PrimitiveHandle> {
        self.resolver.resolve_core_for_enqueue(spelling)
    }

    pub fn resolve_extension_binding(&self, name: &str) -> Option<PrimitiveHandle> {
        self.resolver.resolve_extension_binding(name)
    }
}
