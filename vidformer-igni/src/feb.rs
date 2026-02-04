use std::collections::BTreeMap;

use num_rational::Rational64;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
enum InlineLiteral {
    Int(i32),
    Bool(bool),
    Float(f32),
    ListIntEmpty,
    ListIntSingle(i16),
    ListIntPair(i16, i16),
    ListIntTriple(i16, i16, i16),
}

impl InlineLiteral {
    fn to_int(&self) -> i64 {
        match self {
            InlineLiteral::Int(i) => {
                // Upper byte should be 0x00, lowest 32 bits should be the integer
                *i as i64 & 0xFFFFFFFF
            }
            InlineLiteral::Bool(b) => {
                // Upper byte should be 0x01, lowest bit should be the boolean value
                0x01000000_00000000 | (*b as i64)
            }
            InlineLiteral::Float(f) => {
                // Upper byte should be 0x02, lowest 32 bits should be the float bits
                0x02000000_00000000 | (f.to_bits() as i64)
            }
            InlineLiteral::ListIntEmpty => {
                // Upper byte should be 0x03
                0x03000000_00000000
            }
            InlineLiteral::ListIntSingle(i) => {
                // Upper byte should be 0x04, lowest 16 bits should be the integer
                0x04000000_00000000 | (*i as i64 & 0xFFFF)
            }
            InlineLiteral::ListIntPair(i1, i2) => {
                // Upper byte should be 0x05, lowest 32 bits should be the two integers
                0x05000000_00000000 | (*i1 as i64 & 0xFFFF) << 16 | (*i2 as i64 & 0xFFFF)
            }
            InlineLiteral::ListIntTriple(i1, i2, i3) => {
                // Upper byte should be 0x06, lowest 48 bits should be the three integers
                0x06000000_00000000
                    | (*i1 as i64 & 0xFFFF) << 32
                    | (*i2 as i64 & 0xFFFF) << 16
                    | (*i3 as i64 & 0xFFFF)
            }
        }
    }

    fn from_int(i: i64) -> Result<Self, String> {
        let upper_byte = (i >> 56) as u8;
        let lower_bytes = i & 0x00FF_FFFF_FFFF_FFFF;
        match upper_byte {
            0x00 => Ok(InlineLiteral::Int((lower_bytes & 0xFFFFFFFF) as i32)),
            0x01 => Ok(InlineLiteral::Bool(lower_bytes & 0x1 != 0)),
            0x02 => Ok(InlineLiteral::Float(f32::from_bits(
                (lower_bytes & 0xFFFFFFFF) as u32,
            ))),
            0x03 => Ok(InlineLiteral::ListIntEmpty),
            0x04 => Ok(InlineLiteral::ListIntSingle((lower_bytes & 0xFFFF) as i16)),
            0x05 => Ok(InlineLiteral::ListIntPair(
                ((lower_bytes >> 16) & 0xFFFF) as i16,
                (lower_bytes & 0xFFFF) as i16,
            )),
            0x06 => Ok(InlineLiteral::ListIntTriple(
                ((lower_bytes >> 32) & 0xFFFF) as i16,
                ((lower_bytes >> 16) & 0xFFFF) as i16,
                (lower_bytes & 0xFFFF) as i16,
            )),
            _ => Err(format!("Invalid expr upper byte: 0x{:02X}", upper_byte)),
        }
    }

    fn get_expr(&self) -> Result<vidformer::sir::Expr, String> {
        match self {
            InlineLiteral::Int(i) => Ok(vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(
                *i as i64,
            ))),
            InlineLiteral::Bool(b) => Ok(vidformer::sir::Expr::Data(
                vidformer::sir::DataExpr::Bool(*b),
            )),
            InlineLiteral::Float(f) => Ok(vidformer::sir::Expr::Data(
                vidformer::sir::DataExpr::Float(*f as f64),
            )),
            InlineLiteral::ListIntEmpty => Ok(vidformer::sir::Expr::Data(
                vidformer::sir::DataExpr::List(Vec::new()),
            )),
            InlineLiteral::ListIntSingle(i) => Ok(vidformer::sir::Expr::Data(
                vidformer::sir::DataExpr::List(vec![vidformer::sir::Expr::Data(
                    vidformer::sir::DataExpr::Int(*i as i64),
                )]),
            )),
            InlineLiteral::ListIntPair(i1, i2) => Ok(vidformer::sir::Expr::Data(
                vidformer::sir::DataExpr::List(vec![
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(*i1 as i64)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(*i2 as i64)),
                ]),
            )),
            InlineLiteral::ListIntTriple(i1, i2, i3) => Ok(vidformer::sir::Expr::Data(
                vidformer::sir::DataExpr::List(vec![
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(*i1 as i64)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(*i2 as i64)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(*i3 as i64)),
                ]),
            )),
        }
    }
}

#[derive(Debug, PartialEq)]
enum FrameExprBlock {
    InlineLiteral(InlineLiteral),
    RefLiteral(u32),
    Func {
        len_args: u8,
        len_kwargs: u8,
        func_idx: u16,
    },
    List {
        len: u32,
    },
    SourceILoc {
        source_idx: u16,
        t: u32,
    },
    SourceFrac {
        source_idx: u16,
        source_frac_idx: u32,
    },
    Expr {
        expr_idx: u32,
    },
    KwargKey {
        key_idx: u32,
    },
}

impl FrameExprBlock {
    fn to_int(&self) -> i64 {
        match self {
            FrameExprBlock::InlineLiteral(inline_literal) => inline_literal.to_int(),
            FrameExprBlock::RefLiteral(i) => {
                // Upper byte should be 0x40, lowest 32 bits should be the literal index
                0x40000000_00000000 | (*i as i64)
            }
            FrameExprBlock::Func {
                len_args,
                len_kwargs,
                func_idx,
            } => {
                // Upper byte should be 0x41, next 24 bits should be skipped, next 8 bits should be len_args, next 8 bits should be len_kwargs, next 16 bits should be func_idx
                0x41000000_00000000
                    | (*len_args as i64) << 24
                    | (*len_kwargs as i64) << 16
                    | (*func_idx as i64)
            }
            FrameExprBlock::List { len } => {
                // Upper byte should be 0x42, lowest 32 bits should be the list length
                0x42000000_00000000 | (*len as i64)
            }
            FrameExprBlock::SourceILoc { source_idx, t } => {
                // Upper bytes should be 0x4300, next 16 bits should be source_idx, next 32 bits should be t
                0x43000000_00000000 | (*source_idx as i64) << 32 | (*t as i64)
            }
            FrameExprBlock::SourceFrac {
                source_idx,
                source_frac_idx,
            } => {
                // Upper bytes should be 0x4400, next 16 bits should be source_idx, next 32 bits should be source_frac_idx
                0x44000000_00000000 | (*source_idx as i64) << 32 | (*source_frac_idx as i64)
            }
            FrameExprBlock::Expr { expr_idx } => {
                // Upper bytes should be 0x4500, lowest 32 bits should be expr_idx
                0x45000000_00000000 | (*expr_idx as i64)
            }
            FrameExprBlock::KwargKey { key_idx } => {
                // Upper bytes should be 0x4600, lowest 32 bits should be key_idx
                0x46000000_00000000 | (*key_idx as i64)
            }
        }
    }

    fn from_int(i: i64) -> Result<Self, String> {
        let upper_byte = (i >> 56) as u8;
        match upper_byte {
            _ if upper_byte < 0x40 => InlineLiteral::from_int(i).map(FrameExprBlock::InlineLiteral),
            0x40 => Ok(FrameExprBlock::RefLiteral((i & 0x00000000_FFFFFFFF) as u32)),
            0x41 => {
                let len_args = ((i >> 24) & 0x000000FF) as u8;
                let len_kwargs = ((i >> 16) & 0x000000FF) as u8;
                let func_idx = (i & 0x0000FFFF) as u16;
                Ok(FrameExprBlock::Func {
                    len_args,
                    len_kwargs,
                    func_idx,
                })
            }
            0x42 => {
                let len = (i & 0x00000000_FFFFFFFF) as u32;
                Ok(FrameExprBlock::List { len })
            }
            0x43 => {
                let source_idx = ((i >> 32) & 0x0000FFFF) as u16;
                let t = (i & 0x00000000_FFFFFFFF) as u32;
                Ok(FrameExprBlock::SourceILoc { source_idx, t })
            }
            0x44 => {
                let source_idx = ((i >> 32) & 0x0000FFFF) as u16;
                let source_frac_idx = (i & 0x00000000_FFFFFFFF) as u32;
                Ok(FrameExprBlock::SourceFrac {
                    source_idx,
                    source_frac_idx,
                })
            }
            0x45 => {
                let expr_idx = (i & 0x00000000_FFFFFFFF) as u32;
                Ok(FrameExprBlock::Expr { expr_idx })
            }
            0x46 => {
                let key_idx = (i & 0x00000000_FFFFFFFF) as u32;
                Ok(FrameExprBlock::KwargKey { key_idx })
            }
            _ => Err(format!("Invalid expr upper byte: 0x{:02X}", upper_byte)),
        }
    }
}

fn expr_coded_as_scalar(expr: &vidformer::sir::Expr) -> bool {
    match expr {
        vidformer::sir::Expr::Frame(vidformer::sir::FrameExpr::Source(_)) => true,
        vidformer::sir::Expr::Frame(vidformer::sir::FrameExpr::Filter(_)) => false,
        vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Bool(_)) => true,
        vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(_)) => true,
        vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Float(_)) => true,
        vidformer::sir::Expr::Data(vidformer::sir::DataExpr::List(list)) => {
            if list.len() > 3 {
                false
            } else {
                list.iter().all(|member| match member {
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(i)) => {
                        *i >= i16::MIN as i64 && *i <= i16::MAX as i64
                    }
                    _ => false,
                })
            }
        }
        vidformer::sir::Expr::Data(vidformer::sir::DataExpr::String(_)) => true,
        vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Bytes(_)) => true,
    }
}

#[derive(Debug)]
enum SubExprValue {
    ScalarCoded(vidformer::sir::Expr),
    OutOfBand(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameBlock {
    functions: Vec<String>,
    literals: Vec<vidformer::sir::DataExpr>,
    sources: Vec<String>,
    kwarg_keys: Vec<String>,
    source_fracs: Vec<i64>,
    exprs: Vec<i64>,
    frame_exprs: Vec<i64>,
}

impl FrameBlock {
    pub fn new() -> Self {
        FrameBlock {
            functions: Vec::new(),
            literals: Vec::new(),
            sources: Vec::new(),
            kwarg_keys: Vec::new(),
            source_fracs: Vec::new(),
            exprs: Vec::new(),
            frame_exprs: Vec::new(),
        }
    }

    pub fn insert_frame(&mut self, frame: &vidformer::sir::FrameExpr) -> Result<(), String> {
        let expr_idx = self.insert_frame_expr(frame)?;
        self.frame_exprs.push(expr_idx as i64);
        Ok(())
    }

    fn insert_frame_expr(
        &mut self,
        frame_expr: &vidformer::sir::FrameExpr,
    ) -> Result<usize, String> {
        match frame_expr {
            vidformer::sir::FrameExpr::Source(source) => {
                self.sources.push(source.video().to_string());
                let source_expr = match source.index() {
                    vidformer::sir::IndexConst::ILoc(i) => FrameExprBlock::SourceILoc {
                        source_idx: self.sources.len() as u16 - 1,
                        t: *i as u32,
                    },
                    vidformer::sir::IndexConst::T(t) => {
                        self.source_fracs.push(*t.numer());
                        self.source_fracs.push(*t.denom());
                        FrameExprBlock::SourceFrac {
                            source_idx: self.sources.len() as u16 - 1,
                            source_frac_idx: self.source_fracs.len() as u32 - 2,
                        }
                    }
                };
                self.exprs.push(source_expr.to_int());
                Ok(self.exprs.len() - 1)
            }
            vidformer::sir::FrameExpr::Filter(filter) => {
                let func_idx = self
                    .functions
                    .iter()
                    .position(|f| f == &filter.name)
                    .unwrap_or_else(|| {
                        self.functions.push(filter.name.to_string());
                        self.functions.len() - 1
                    }) as u16;
                if filter.args.len() > 255 {
                    return Err("Too many filter args".to_string());
                }
                if filter.kwargs.len() > 255 {
                    return Err("Too many filter kwargs".to_string());
                }
                let len_args: u8 = filter.args.len() as u8;
                let len_kwargs = filter.kwargs.len() as u8;
                let func_expr = FrameExprBlock::Func {
                    len_args,
                    len_kwargs,
                    func_idx,
                };

                let mut sub_args: Vec<SubExprValue> = Vec::with_capacity(filter.args.len());
                for arg in &filter.args {
                    if expr_coded_as_scalar(arg) {
                        sub_args.push(SubExprValue::ScalarCoded(arg.clone()));
                    } else {
                        let idx = self.insert_expr(arg)?;
                        sub_args.push(SubExprValue::OutOfBand(idx));
                    }
                }
                let mut sub_kwargs: BTreeMap<String, SubExprValue> = BTreeMap::new();
                for (kwkey, kwarg) in &filter.kwargs {
                    if expr_coded_as_scalar(kwarg) {
                        sub_kwargs
                            .insert(kwkey.to_string(), SubExprValue::ScalarCoded(kwarg.clone()));
                    } else {
                        let idx = self.insert_expr(kwarg)?;
                        sub_kwargs.insert(kwkey.to_string(), SubExprValue::OutOfBand(idx));
                    }
                }
                let out_idx = self.exprs.len();
                self.exprs.push(func_expr.to_int());

                for sub_arg in sub_args {
                    match sub_arg {
                        SubExprValue::ScalarCoded(expr) => {
                            self.insert_expr(&expr)?;
                        }
                        SubExprValue::OutOfBand(idx) => {
                            self.exprs.push(
                                FrameExprBlock::Expr {
                                    expr_idx: idx as u32,
                                }
                                .to_int(),
                            );
                        }
                    }
                }
                for (kwkey, sub_kwarg) in sub_kwargs {
                    let key_idx = self
                        .kwarg_keys
                        .iter()
                        .position(|k| k == &kwkey)
                        .unwrap_or_else(|| {
                            self.kwarg_keys.push(kwkey.to_string());
                            self.kwarg_keys.len() - 1
                        }) as u32;
                    let kwarg_key = FrameExprBlock::KwargKey { key_idx };
                    self.exprs.push(kwarg_key.to_int());
                    match sub_kwarg {
                        SubExprValue::ScalarCoded(expr) => {
                            self.insert_expr(&expr)?;
                        }
                        SubExprValue::OutOfBand(idx) => {
                            self.exprs.push(
                                FrameExprBlock::Expr {
                                    expr_idx: idx as u32,
                                }
                                .to_int(),
                            );
                        }
                    }
                }
                Ok(out_idx)
            }
        }
    }

    fn insert_data_expr(&mut self, data_expr: &vidformer::sir::DataExpr) -> Result<usize, String> {
        match data_expr {
            &vidformer::sir::DataExpr::Bool(b) => {
                let inline_literal = InlineLiteral::Bool(b);
                self.exprs
                    .push(FrameExprBlock::InlineLiteral(inline_literal).to_int());
                Ok(self.exprs.len() - 1)
            }
            &vidformer::sir::DataExpr::Int(i) => {
                if i < i32::MIN as i64 || i > i32::MAX as i64 {
                    self.literals.push(data_expr.clone());
                    let ref_literal = FrameExprBlock::RefLiteral(self.literals.len() as u32 - 1);
                    self.exprs.push(ref_literal.to_int());
                } else {
                    let inline_literal = InlineLiteral::Int(i as i32);
                    self.exprs
                        .push(FrameExprBlock::InlineLiteral(inline_literal).to_int());
                }
                Ok(self.exprs.len() - 1)
            }
            &vidformer::sir::DataExpr::Float(f) => {
                let inline_literal = InlineLiteral::Float(f as f32);
                self.exprs
                    .push(FrameExprBlock::InlineLiteral(inline_literal).to_int());
                Ok(self.exprs.len() - 1)
            }
            vidformer::sir::DataExpr::List(list) => {
                if list.len() > u32::MAX as usize {
                    return Err("List too long".to_string());
                }
                let len = list.len() as u32;
                match (len, list.first(), list.get(1), list.get(2)) {
                    (0, None, None, None) => {
                        let inline_literal = InlineLiteral::ListIntEmpty;
                        self.exprs
                            .push(FrameExprBlock::InlineLiteral(inline_literal).to_int());
                        Ok(self.exprs.len() - 1)
                    }
                    (
                        1,
                        Some(vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(i1))),
                        None,
                        None,
                    ) if *i1 >= i16::MIN as i64 && *i1 <= i16::MAX as i64 => {
                        let inline_literal = InlineLiteral::ListIntSingle(*i1 as i16);
                        self.exprs
                            .push(FrameExprBlock::InlineLiteral(inline_literal).to_int());
                        Ok(self.exprs.len() - 1)
                    }
                    (
                        2,
                        Some(vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(i1))),
                        Some(vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(i2))),
                        None,
                    ) if *i1 >= i16::MIN as i64
                        && *i1 <= i16::MAX as i64
                        && *i2 >= i16::MIN as i64
                        && *i2 <= i16::MAX as i64 =>
                    {
                        let inline_literal = InlineLiteral::ListIntPair(*i1 as i16, *i2 as i16);
                        self.exprs
                            .push(FrameExprBlock::InlineLiteral(inline_literal).to_int());
                        Ok(self.exprs.len() - 1)
                    }
                    (
                        3,
                        Some(vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(i1))),
                        Some(vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(i2))),
                        Some(vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(i3))),
                    ) if *i1 >= i16::MIN as i64
                        && *i1 <= i16::MAX as i64
                        && *i2 >= i16::MIN as i64
                        && *i2 <= i16::MAX as i64
                        && *i3 >= i16::MIN as i64
                        && *i3 <= i16::MAX as i64 =>
                    {
                        let inline_literal =
                            InlineLiteral::ListIntTriple(*i1 as i16, *i2 as i16, *i3 as i16);
                        self.exprs
                            .push(FrameExprBlock::InlineLiteral(inline_literal).to_int());
                        Ok(self.exprs.len() - 1)
                    }
                    _ => {
                        let mut sub_exprs: Vec<SubExprValue> = Vec::with_capacity(len as usize);
                        for expr in list {
                            if expr_coded_as_scalar(expr) {
                                sub_exprs.push(SubExprValue::ScalarCoded(expr.clone()));
                            } else {
                                let idx = self.insert_expr(expr)?;
                                sub_exprs.push(SubExprValue::OutOfBand(idx));
                            }
                        }
                        let out_idx = self.exprs.len();
                        self.exprs.push(FrameExprBlock::List { len }.to_int());
                        for sub_expr in sub_exprs {
                            match sub_expr {
                                SubExprValue::ScalarCoded(expr) => {
                                    self.insert_expr(&expr)?;
                                }
                                SubExprValue::OutOfBand(idx) => {
                                    self.exprs.push(
                                        FrameExprBlock::Expr {
                                            expr_idx: idx as u32,
                                        }
                                        .to_int(),
                                    );
                                }
                            }
                        }
                        Ok(out_idx)
                    }
                }
            }
            &vidformer::sir::DataExpr::String(_) => {
                self.literals.push(data_expr.clone());
                let ref_literal = FrameExprBlock::RefLiteral(self.literals.len() as u32 - 1);
                self.exprs.push(ref_literal.to_int());
                Ok(self.exprs.len() - 1)
            }
            &vidformer::sir::DataExpr::Bytes(_) => {
                self.literals.push(data_expr.clone());
                let ref_literal = FrameExprBlock::RefLiteral(self.literals.len() as u32 - 1);
                self.exprs.push(ref_literal.to_int());
                Ok(self.exprs.len() - 1)
            }
        }
    }

    fn insert_expr(&mut self, expr: &vidformer::sir::Expr) -> Result<usize, String> {
        match expr {
            vidformer::sir::Expr::Frame(frame_expr) => self.insert_frame_expr(frame_expr),
            vidformer::sir::Expr::Data(data_expr) => self.insert_data_expr(data_expr),
        }
    }

    fn get_expr(&self, idx: usize, member: bool) -> Result<vidformer::sir::Expr, String> {
        let target_expr = self.exprs.get(idx).ok_or("Expr index out of bounds")?;
        let target_expr = FrameExprBlock::from_int(*target_expr)?;
        match target_expr {
            FrameExprBlock::Expr { expr_idx } if member => self.get_expr(expr_idx as usize, false),
            FrameExprBlock::Expr { .. } => Err(format!(
                "Expr at pos {} is not a Function or List member",
                idx
            )),
            FrameExprBlock::InlineLiteral(inline_literal) => inline_literal.get_expr(),
            FrameExprBlock::RefLiteral(i) => Ok(vidformer::sir::Expr::Data(
                self.literals
                    .get(i as usize)
                    .ok_or("Literal index out of bounds")?
                    .clone(),
            )),
            FrameExprBlock::List { .. } if member => {
                Err("A list can't be a direct member".to_string())
            }
            FrameExprBlock::List { len } => {
                let mut list = Vec::with_capacity(len as usize);
                for i in 0..len {
                    let expr = self.get_expr(idx + 1 + i as usize, true)?;
                    list.push(expr);
                }
                Ok(vidformer::sir::Expr::Data(vidformer::sir::DataExpr::List(
                    list,
                )))
            }
            FrameExprBlock::KwargKey { .. } => {
                Err(format!("Pos {} needs to be an expr, not kwarg key", idx))
            }
            FrameExprBlock::SourceILoc { .. } | FrameExprBlock::SourceFrac { .. } => {
                self.get_frame(idx).map(vidformer::sir::Expr::Frame)
            }
            FrameExprBlock::Func { .. } if member => {
                Err("A function can't be a direct member".to_string())
            }
            FrameExprBlock::Func { .. } => self.get_frame(idx).map(vidformer::sir::Expr::Frame),
        }
    }

    fn get_frame(&self, idx: usize) -> Result<vidformer::sir::FrameExpr, String> {
        let target_expr = self.exprs.get(idx).ok_or("Expr index out of bounds")?;
        let target_expr = FrameExprBlock::from_int(*target_expr)?;
        match target_expr {
            FrameExprBlock::Expr { .. } => {
                Err(format!("Pos {} needs to be a Function or Source", idx))
            }
            FrameExprBlock::InlineLiteral(_) => {
                Err(format!("Pos {} needs to be a Function or Source", idx))
            }
            FrameExprBlock::RefLiteral(_) => {
                Err(format!("Pos {} needs to be a Function or Source", idx))
            }
            FrameExprBlock::List { .. } => {
                Err(format!("Pos {} needs to be a Function or Source", idx))
            }
            FrameExprBlock::KwargKey { .. } => {
                Err(format!("Pos {} needs to be a Function or Source", idx))
            }
            FrameExprBlock::SourceILoc { source_idx, t } => {
                let source = self
                    .sources
                    .get(source_idx as usize)
                    .ok_or("Source index out of bounds")?;
                Ok(vidformer::sir::FrameExpr::Source(
                    vidformer::sir::FrameSource::new(
                        source.to_string(),
                        vidformer::sir::IndexConst::ILoc(t as usize),
                    ),
                ))
            }
            FrameExprBlock::SourceFrac {
                source_idx,
                source_frac_idx,
            } => {
                let source = self
                    .sources
                    .get(source_idx as usize)
                    .ok_or("Source index out of bounds")?;
                let t = Rational64::new(
                    *self
                        .source_fracs
                        .get(source_frac_idx as usize * 2)
                        .ok_or("Source frac index out of bounds")?,
                    *self
                        .source_fracs
                        .get(source_frac_idx as usize * 2 + 1)
                        .ok_or("Source frac index out of bounds")?,
                );
                Ok(vidformer::sir::FrameExpr::Source(
                    vidformer::sir::FrameSource::new(
                        source.to_string(),
                        vidformer::sir::IndexConst::T(t),
                    ),
                ))
            }
            FrameExprBlock::Func {
                len_args,
                len_kwargs,
                func_idx,
            } => {
                let func_name = self
                    .functions
                    .get(func_idx as usize)
                    .ok_or("Function index out of bounds")?;
                let mut args = Vec::with_capacity(len_args as usize);
                let mut kwargs = std::collections::BTreeMap::new();
                for i in 0..len_args as usize {
                    args.push(self.get_expr(idx + 1 + i, true)?);
                }
                for i in 0..len_kwargs as usize {
                    let key = match FrameExprBlock::from_int(
                        *self
                            .exprs
                            .get(idx + 1 + len_args as usize + i * 2)
                            .ok_or("Kwarg key index out of bounds".to_string())?,
                    )? {
                        FrameExprBlock::KwargKey { key_idx } => self
                            .kwarg_keys
                            .get(key_idx as usize)
                            .ok_or("Kwarg key index out of bounds")?,
                        _ => {
                            return Err(format!(
                                "Pos {} needs to be a Kwarg key",
                                idx + 1 + len_args as usize + i * 2
                            ))
                        }
                    };
                    let value = self.get_expr(idx + 1 + len_args as usize + i * 2 + 1, true)?;
                    kwargs.insert(key.to_string(), value);
                }
                Ok(vidformer::sir::FrameExpr::Filter(
                    vidformer::sir::FilterExpr {
                        name: func_name.to_string(),
                        args,
                        kwargs,
                    },
                ))
            }
        }
    }

    pub fn frames(&self) -> Result<Vec<vidformer::sir::FrameExpr>, String> {
        self.frame_exprs
            .iter()
            .map(|i| self.get_frame(*i as usize))
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_empty_frame_block() {
        let frame_block_str = r#"{
            "functions": [],
            "literals": [],
            "sources": [],
            "kwarg_keys": [],
            "source_fracs": [],
            "exprs": [],
            "frame_exprs": []
        }"#;
        let _: FrameBlock = serde_json::from_str(frame_block_str).unwrap();
    }

    #[test]
    fn test_insert_frame_simple_1_frame() {
        let mut frame_block = FrameBlock::new();
        let frame_expr = vidformer::sir::FrameExpr::Source(vidformer::sir::FrameSource::new(
            "video.mp4".to_string(),
            vidformer::sir::IndexConst::ILoc(0),
        ));
        frame_block.insert_frame(&frame_expr).unwrap();

        assert_eq!(vec![frame_expr], frame_block.frames().unwrap());
    }

    #[test]
    fn test_insert_frame_simple_2_frames() {
        let mut frame_block = FrameBlock::new();
        let frame_expr_1 = vidformer::sir::FrameExpr::Source(vidformer::sir::FrameSource::new(
            "video.mp4".to_string(),
            vidformer::sir::IndexConst::ILoc(0),
        ));
        let frame_expr_2 = vidformer::sir::FrameExpr::Source(vidformer::sir::FrameSource::new(
            "video.mp4".to_string(),
            vidformer::sir::IndexConst::ILoc(1),
        ));
        frame_block.insert_frame(&frame_expr_1).unwrap();
        frame_block.insert_frame(&frame_expr_2).unwrap();

        assert_eq!(
            vec![frame_expr_1, frame_expr_2],
            frame_block.frames().unwrap()
        );
    }

    #[test]
    fn test_insert_frame_filter() {
        let mut frame_block = FrameBlock::new();
        let frame_expr = vidformer::sir::FrameExpr::Filter(vidformer::sir::FilterExpr {
            name: "filter".to_string(),
            args: vec![vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(0))],
            kwargs: std::collections::BTreeMap::new(),
        });
        frame_block.insert_frame(&frame_expr).unwrap();
        assert_eq!(vec![frame_expr], frame_block.frames().unwrap());
    }

    #[test]
    fn test_complex_frame_block() {
        let mut frame_block = FrameBlock::new();
        let frame_expr_1 = vidformer::sir::FrameExpr::Source(vidformer::sir::FrameSource::new(
            "video.mp4".to_string(),
            vidformer::sir::IndexConst::ILoc(0),
        ));
        let frame_expr_2 = vidformer::sir::FrameExpr::Filter(vidformer::sir::FilterExpr {
            name: "filter".to_string(),
            args: vec![vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(0))],
            kwargs: std::collections::BTreeMap::new(),
        });
        frame_block.insert_frame(&frame_expr_1).unwrap();
        frame_block.insert_frame(&frame_expr_2).unwrap();

        assert_eq!(
            vec![frame_expr_1, frame_expr_2],
            frame_block.frames().unwrap()
        );
    }

    #[test]
    fn test_with_kwargs() {
        let mut frame_block = FrameBlock::new();
        let frame_expr = vidformer::sir::FrameExpr::Filter(vidformer::sir::FilterExpr {
            name: "filter".to_string(),
            args: vec![vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(0))],
            kwargs: vec![(
                "key".to_string(),
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(0)),
            )]
            .into_iter()
            .collect(),
        });
        frame_block.insert_frame(&frame_expr).unwrap();
        assert_eq!(vec![frame_expr], frame_block.frames().unwrap());
    }

    #[test]
    fn test_multiple_args() {
        let mut frame_block = FrameBlock::new();
        let mut frame_exprs = Vec::new();
        for i in 0..10 {
            let frame_expr = vidformer::sir::FrameExpr::Filter(vidformer::sir::FilterExpr {
                name: "filter".to_string(),
                args: (0..i)
                    .map(|j| {
                        vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(j as i64 + 3))
                    })
                    .collect(),
                kwargs: std::collections::BTreeMap::new(),
            });
            frame_block.insert_frame(&frame_expr).unwrap();
            frame_exprs.push(frame_expr);
        }
        assert_eq!(frame_exprs, frame_block.frames().unwrap());
    }

    #[test]
    fn test_multiple_kwargs() {
        let mut frame_block = FrameBlock::new();
        let mut frame_exprs = Vec::new();
        for i in 0..10 {
            let frame_expr = vidformer::sir::FrameExpr::Filter(vidformer::sir::FilterExpr {
                name: "filter".to_string(),
                args: vec![],
                kwargs: (0..i)
                    .map(|j| {
                        (
                            format!("kwarg{}", j),
                            vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(j as i64 + 3)),
                        )
                    })
                    .into_iter()
                    .collect(),
            });
            frame_block.insert_frame(&frame_expr).unwrap();
            frame_exprs.push(frame_expr);
        }
        assert_eq!(frame_exprs, frame_block.frames().unwrap());
    }

    #[test]
    fn test_negative_ints() {
        let mut frame_block = FrameBlock::new();
        let frame_expr = vidformer::sir::FrameExpr::Filter(vidformer::sir::FilterExpr {
            name: "filter".to_string(),
            args: vec![
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-3)),
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-100_000_000_000)),
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::List(vec![
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-8)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-1024)),
                ])),
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::List(vec![
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-1)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-2)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-3)),
                ])),
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::List(vec![
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-1)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-2000)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-3)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(-8192)),
                ])),
            ],
            kwargs: std::collections::BTreeMap::new(),
        });
        frame_block.insert_frame(&frame_expr).unwrap();
        assert_eq!(vec![frame_expr], frame_block.frames().unwrap());
    }

    #[test]
    fn test_lists() {
        // Insert 5 frames, each with n number of elements in a list argument to a filter
        let mut frame_block = FrameBlock::new();
        let mut frame_exprs = Vec::new();
        for i in 1..10 {
            let frame_expr = vidformer::sir::FrameExpr::Filter(vidformer::sir::FilterExpr {
                name: "filter".to_string(),
                args: vec![vidformer::sir::Expr::Data(vidformer::sir::DataExpr::List(
                    (0..i)
                        .map(|j| {
                            vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(j as i64 + 3))
                        })
                        .collect(),
                ))],
                kwargs: std::collections::BTreeMap::new(),
            });
            frame_block.insert_frame(&frame_expr).unwrap();
            frame_exprs.push(frame_expr);
        }
        dbg!(&frame_block, &frame_exprs);
        assert_eq!(frame_exprs, frame_block.frames().unwrap());
    }

    #[test]
    fn test_nested_lists() {
        // Insert 5 frames, each with n number of elements in a list argument to a filter
        let mut frame_block = FrameBlock::new();
        let mut frame_exprs = Vec::new();
        for i in 0..5 {
            let frame_expr = vidformer::sir::FrameExpr::Filter(vidformer::sir::FilterExpr {
                name: "filter".to_string(),
                args: vec![vidformer::sir::Expr::Data(vidformer::sir::DataExpr::List(
                    (0..i)
                        .map(|j| {
                            vidformer::sir::Expr::Data(vidformer::sir::DataExpr::List(
                                (0..j)
                                    .map(|k| {
                                        vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(
                                            k as i64 + 3,
                                        ))
                                    })
                                    .collect(),
                            ))
                        })
                        .collect(),
                ))],
                kwargs: std::collections::BTreeMap::new(),
            });
            frame_block.insert_frame(&frame_expr).unwrap();
            frame_exprs.push(frame_expr);
        }
        assert_eq!(frame_exprs, frame_block.frames().unwrap());
    }

    #[test]
    fn test_all_data_types() {
        let mut frame_block = FrameBlock::new();
        let frame_expr = vidformer::sir::FrameExpr::Filter(vidformer::sir::FilterExpr {
            name: "filter".to_string(),
            args: vec![
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Bool(true)),
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(3)),
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Float(4.0)), // this float can be exactly represented as an f32
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::List(vec![
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(1)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(2)),
                    vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Int(3)),
                ])),
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::String("hello".to_string())),
                vidformer::sir::Expr::Data(vidformer::sir::DataExpr::Bytes(vec![0x01, 0x02, 0x03])),
            ],
            kwargs: std::collections::BTreeMap::new(),
        });
        frame_block.insert_frame(&frame_expr).unwrap();
        assert_eq!(vec![frame_expr], frame_block.frames().unwrap());
    }
}
