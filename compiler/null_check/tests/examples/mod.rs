//! The spec's examples of components (section 8), shared by the checker's tests and the
//! expansion's (tasks 1d-03b and 1d-03c).

/// Materials the examples name, defined elsewhere in a project.
pub const MATERIALS: &str =
    "material @weathered-oak \"Weathered Oak\"\nmaterial @water \"Water\"\n";

/// The plank of the pier (section 8).
pub const PLANK: &str = r#"component @plank "Plank" {
  param length 2.4m
  param state solid { one-of [solid liquid] }
  object @plank "Plank" {
    data mesh @plank-mesh
    if "state == solid" { material @weathered-oak }
    else {
      material @water
      modifier @fluid fluid { fluid-type flow }
    }
  }
  mesh @plank-mesh "Plank" {
    primitive cube size=1m scale=[length/1m 0.2 0.04]
  }
}
"#;

/// The pier: twenty planks, plank 7 liquid.
pub const PIER: &str = r#"collection @pier "Pier" {
  use @plank @plank count=20 {
    location [0m n*25cm 0.35m]
  }
  override @plank.7 { state liquid }
}
"#;

/// The branch whose copies count down.
pub const BRANCH: &str = r#"component @branch "Branch" {
  param depth 4 { countdown }
  param length 1m
  object @branch "Branch" {
    data mesh @segment
  }
  mesh @segment "Segment" { primitive cylinder radius=length*0.05 depth=length }
  if "depth > 0" {
    use @branch @left {
      depth "depth - 1"
      length length*0.7
      parent @branch
      location [0m 0m length]
      rotation-euler [0deg 35deg 0deg]
    }
    use @branch @right {
      depth "depth - 1"
      length length*0.7
      parent @branch
      location [0m 0m length]
      rotation-euler [0deg -35deg 0deg]
    }
  }
}
"#;
