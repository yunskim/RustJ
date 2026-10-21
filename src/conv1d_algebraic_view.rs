//! Minimal Conv1D semantic registry prototype. Not wired into production optimizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conv1dSpec { pub stride: usize, pub padding: usize, pub dilation: usize }
impl Conv1dSpec {
    pub const VALID: Self = Self { stride: 1, padding: 0, dilation: 1 };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode { Forward, InputAdjoint, WeightAdjoint }
/// Symbolic index variables; no runtime array elements are exposed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis { Output, Kernel, Input }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand { Input, Weight, Cotangent }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Index {
    Axis(Axis),
    OutputPlusKernel { stride: usize, dilation: usize, padding: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Read { operand: Operand, index: Index },
    Mul(Box<Expr>, Box<Expr>),
    Sum { axis: Axis, body: Box<Expr> },
    /// Restricted inverse-index accumulation for the input partial adjoint.
    ScatterSum { output: Axis, kernel: Axis, body: Box<Expr> },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicView { pub mode: ViewMode, pub expr: Expr }
fn read(operand: Operand, index: Index) -> Expr { Expr::Read { operand, index } }
fn mul(a: Expr, b: Expr) -> Expr { Expr::Mul(Box::new(a), Box::new(b)) }
pub fn view(mode: ViewMode, spec: Conv1dSpec) -> Option<AlgebraicView> {
    if spec != Conv1dSpec::VALID { return None }
    let window = Index::OutputPlusKernel {
        stride: spec.stride, dilation: spec.dilation, padding: spec.padding
    };
    let expr = match mode {
        ViewMode::Forward => Expr::Sum { axis: Axis::Kernel, body: Box::new(mul(
            read(Operand::Input, window), read(Operand::Weight, Index::Axis(Axis::Kernel)))) },
        ViewMode::InputAdjoint => Expr::ScatterSum {
            output: Axis::Output, kernel: Axis::Kernel, body: Box::new(mul(
                read(Operand::Cotangent, Index::Axis(Axis::Output)),
                read(Operand::Weight, Index::Axis(Axis::Kernel)))) },
        ViewMode::WeightAdjoint => Expr::Sum { axis: Axis::Output, body: Box::new(mul(
            read(Operand::Input, window),
            read(Operand::Cotangent, Index::Axis(Axis::Output)))) },
    };
    Some(AlgebraicView { mode, expr })
}
/// Match a symbolic contraction, rather than checking only a mode/spec flag.
fn forward_contraction(expr: &Expr) -> Option<(usize, usize, usize)> {
    let Expr::Sum { axis: Axis::Kernel, body } = expr else { return None };
    let Expr::Mul(left, right) = body.as_ref() else { return None };
    let Expr::Read { operand: Operand::Input,
        index: Index::OutputPlusKernel { stride, padding, dilation } } = left.as_ref() else { return None };
    let Expr::Read { operand: Operand::Weight, index: Index::Axis(Axis::Kernel) } = right.as_ref() else { return None };
    Some((*stride, *padding, *dilation))
}
#[derive(Clone, Debug, PartialEq)]
pub enum Graph {
    Input(Vec<f64>),
    Weight(Vec<f64>),
    Add(Box<Graph>, Box<Graph>),
    Conv1d { input: Box<Graph>, weight: Box<Graph>, spec: Conv1dSpec },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Logical {
    IndexMapWindow { stride: usize, padding: usize, dilation: usize },
    Multiply,
    ReduceSumKernel,
}
pub fn lower(node: &Graph) -> Option<Vec<Logical>> {
    let Graph::Conv1d { spec, .. } = node else { return None };
    let definition = view(ViewMode::Forward, *spec)?;
    let (stride, padding, dilation) = forward_contraction(&definition.expr)?;
    Some(vec![
        Logical::IndexMapWindow { stride, padding, dilation },
        Logical::Multiply,
        Logical::ReduceSumKernel,
    ])
}
pub fn rewrite_sum_of_convs(node: &Graph) -> Option<Graph> {
    let Graph::Add(a,b) = node else { return None };
    let (Graph::Conv1d { input: x1, weight: w1, spec: s1 },
         Graph::Conv1d { input: x2, weight: w2, spec: s2 }) = (a.as_ref(),b.as_ref()) else { return None };
    // Only permit merging with structurally identical input and matching semantics.
    if x1 != x2 || s1 != s2 || view(ViewMode::Forward,*s1).and_then(|v| forward_contraction(&v.expr)).is_none() { return None }
    let (Graph::Weight(wa), Graph::Weight(wb)) = (w1.as_ref(),w2.as_ref()) else { return None };
    if wa.len()!=wb.len() || wa.is_empty() { return None }
    Some(Graph::Conv1d { input: x1.clone(), weight: Box::new(Graph::Weight(
        wa.iter().zip(wb).map(|(a,b)|a+b).collect())), spec: *s1 })
}
pub fn eval(node: &Graph) -> Option<Vec<f64>> {
    match node {
        Graph::Input(v) | Graph::Weight(v) => Some(v.clone()),
        Graph::Add(a,b) => {
            let (a,b)=(eval(a)?,eval(b)?);
            (a.len()==b.len()).then(|| a.iter().zip(b).map(|(x,y)|x+y).collect())
        }
        Graph::Conv1d { input,weight,spec } => {
            view(ViewMode::Forward,*spec)?;
            let (x,w)=(eval(input)?,eval(weight)?);
            if w.is_empty() || x.len()<w.len() { return None }
            Some((0..=x.len()-w.len()).map(|p|w.iter().enumerate().map(|(k,v)|x[p+k]*v).sum()).collect())
        }
    }
}
/// Execute a registered adjoint view for the restricted single-channel prototype.
/// This is an independent oracle, not production Logical IR execution.
pub fn eval_adjoint(
    mode: ViewMode,
    fixed: &[f64],
    cotangent: &[f64],
    target_len: usize,
    spec: Conv1dSpec,
) -> Option<Vec<f64>> {
    let registered = view(mode, spec)?;
    match registered.expr {
        Expr::Sum { axis: Axis::Kernel, .. } => None,
        Expr::ScatterSum { .. } => {
            if fixed.is_empty() || target_len < fixed.len()
                || cotangent.len() != target_len - fixed.len() + 1 {
                return None;
            }
            let mut dx = vec![0.; target_len];
            for (p, &dy) in cotangent.iter().enumerate() {
                for (k, &w) in fixed.iter().enumerate() {
                    dx[p + k] += dy * w;
                }
            }
            Some(dx)
        }
        Expr::Sum { axis: Axis::Output, .. } => {
            if target_len == 0 || fixed.len() < target_len
                || cotangent.len() != fixed.len() - target_len + 1 {
                return None;
            }
            Some((0..target_len).map(|k| {
                cotangent.iter().enumerate().map(|(p, &dy)| fixed[p + k] * dy).sum()
            }).collect())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_view_rewrite_and_lower() {
        let x=Graph::Input(vec![1.,2.,3.,4.]);
        let mk=|w:Vec<f64>|Graph::Conv1d {input:Box::new(x.clone()),weight:Box::new(Graph::Weight(w)),spec:Conv1dSpec::VALID};
        let original=Graph::Add(Box::new(mk(vec![2.,3.])),Box::new(mk(vec![4.,1.])));
        let optimized=rewrite_sum_of_convs(&original).unwrap();
        assert!(matches!(optimized,Graph::Conv1d { .. }));
        assert_eq!(eval(&original),Some(vec![14.,24.,34.]));
        assert_eq!(eval(&optimized),eval(&original));
        assert_eq!(lower(&optimized),Some(vec![
            Logical::IndexMapWindow{stride:1,padding:0,dilation:1},Logical::Multiply,Logical::ReduceSumKernel]));
    }
    #[test]
    fn adjoint_views_share_contract() {
        let x = [1., 2., 3., 4.];
        let w = [2., 3.];
        let dy = [1., 2., 3.];
        let dx = eval_adjoint(ViewMode::InputAdjoint, &w, &dy, x.len(), Conv1dSpec::VALID).unwrap();
        let dw = eval_adjoint(ViewMode::WeightAdjoint, &x, &dy, w.len(), Conv1dSpec::VALID).unwrap();
        assert_eq!(dx, vec![2., 7., 12., 9.]);
        assert_eq!(dw, vec![14., 20.]);
        let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x,y)| x*y).sum::<f64>();
        let y = eval(&Graph::Conv1d {
            input: Box::new(Graph::Input(x.to_vec())),
            weight: Box::new(Graph::Weight(w.to_vec())),
            spec: Conv1dSpec::VALID,
        }).unwrap();
        assert_eq!(dot(&y, &dy), 88.);
        assert_eq!(dot(&x, &dx), 88.);
        assert_eq!(dot(&w, &dw), 88.);
        assert!(eval_adjoint(ViewMode::InputAdjoint, &w, &dy, 3, Conv1dSpec::VALID).is_none());
        assert!(eval_adjoint(ViewMode::Forward, &w, &dy, 4, Conv1dSpec::VALID).is_none());
    }

    #[test]
    fn reject_incompatible_or_private_weights() {
        let x=Graph::Input(vec![1.,2.,3.]);
        let mk=|w:Graph|Graph::Conv1d {input:Box::new(x.clone()),weight:Box::new(w),spec:Conv1dSpec::VALID};
        assert!(rewrite_sum_of_convs(&Graph::Add(Box::new(mk(Graph::Weight(vec![1.,2.]))),
            Box::new(mk(Graph::Input(vec![3.,4.]))))).is_none());
    }
}
