//! Minimal Conv1D semantic registry prototype. Not wired into production optimizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conv1dSpec { pub stride: usize, pub padding: usize, pub dilation: usize }
impl Conv1dSpec {
    pub const VALID: Self = Self { stride: 1, padding: 0, dilation: 1 };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode { Forward, InputAdjoint, WeightAdjoint }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlgebraicView { pub mode: ViewMode, pub stride: usize, pub padding: usize, pub dilation: usize }
pub fn view(mode: ViewMode, spec: Conv1dSpec) -> Option<AlgebraicView> {
    (spec == Conv1dSpec::VALID).then_some(AlgebraicView {
        mode, stride: spec.stride, padding: spec.padding, dilation: spec.dilation
    })
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
    match node {
        Graph::Conv1d { spec, .. } => {
            let v = view(ViewMode::Forward, *spec)?;
            Some(vec![Logical::IndexMapWindow { stride: v.stride, padding: v.padding, dilation: v.dilation },
                      Logical::Multiply, Logical::ReduceSumKernel])
        }
        _ => None,
    }
}
pub fn rewrite_sum_of_convs(node: &Graph) -> Option<Graph> {
    let Graph::Add(a,b) = node else { return None };
    let (Graph::Conv1d { input: x1, weight: w1, spec: s1 },
         Graph::Conv1d { input: x2, weight: w2, spec: s2 }) = (a.as_ref(),b.as_ref()) else { return None };
    // Only permit merging with structurally identical input and matching semantics.
    if x1 != x2 || s1 != s2 || view(ViewMode::Forward,*s1).is_none() { return None }
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
    fn reject_incompatible_or_private_weights() {
        let x=Graph::Input(vec![1.,2.,3.]);
        let mk=|w:Graph|Graph::Conv1d {input:Box::new(x.clone()),weight:Box::new(w),spec:Conv1dSpec::VALID};
        assert!(rewrite_sum_of_convs(&Graph::Add(Box::new(mk(Graph::Weight(vec![1.,2.]))),
            Box::new(mk(Graph::Input(vec![3.,4.]))))).is_none());
    }
}
