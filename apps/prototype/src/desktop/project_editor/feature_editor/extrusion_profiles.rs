//! Seleção conjunta de áreas da mesma extrusão, independente de corpo ou rascunho.
use super::*;
impl Draft {
    pub(in crate::desktop::project_editor) fn extrusion_profiles(&self) -> Vec<ExtrusionProfile> {
        let first = if let Some(area) = self.face_area() {
            vec![ExtrusionProfile::Face {
                base: area.body,
                face: area.face.clone(),
                boundaries: area.boundaries.clone(),
            }]
        } else if let Some(sketch) = self.sketch() {
            self.region_reference()
                .cloned()
                .or_else(|| self.region.map(RegionRef::Index))
                .map(|region| {
                    region
                        .selections()
                        .iter()
                        .map(|region| {
                            ExtrusionProfile::Sketch(ProfileRef {
                                sketch,
                                region: region.clone(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        first
            .into_iter()
            .chain(self.extra_extrusion_profiles.clone())
            .collect()
    }
    pub(super) fn set_extrusion_profiles(&mut self, mut profiles: Vec<ExtrusionProfile>) {
        self.face_profile = None;
        self.kept_region = None;
        self.sketch = None;
        self.region = None;
        self.auto_preferred = profiles.iter().find_map(ExtrusionProfile::base);
        if !profiles.is_empty() {
            match profiles.remove(0) {
                ExtrusionProfile::Sketch(p) => {
                    self.sketch = Some(p.sketch);
                    self.kept_region = Some(p.region);
                }
                ExtrusionProfile::Face {
                    base,
                    face,
                    boundaries,
                } => {
                    self.face_profile = Some(FaceArea {
                        body: base,
                        face,
                        boundaries,
                    })
                }
            }
        } else {
            self.extrusion_plane = None;
        }
        if let Some(sketch) = self.sketch
            && profiles
                .iter()
                .all(|p| matches!(p, ExtrusionProfile::Sketch(p) if p.sketch == sketch))
            && !profiles.is_empty()
        {
            let mut regions = self.kept_region.take().into_iter().collect::<Vec<_>>();
            regions.extend(profiles.drain(..).filter_map(|p| match p {
                ExtrusionProfile::Sketch(p) => Some(p.region),
                _ => None,
            }));
            self.kept_region = Some(RegionRef::Multiple(regions));
        }
        self.extra_extrusion_profiles = profiles;
        self.interaction = extrusion::Interaction::default();
        self.field = Some(Field::Profile);
    }
    pub(super) fn toggle_extrusion_profile(&mut self, profile: ExtrusionProfile) {
        let mut profiles = self.extrusion_profiles();
        if let Some(index) = profiles.iter().position(|p| p == &profile) {
            profiles.remove(index);
        } else {
            profiles.push(profile);
        }
        self.set_extrusion_profiles(profiles);
    }
    pub(in crate::desktop::project_editor) fn face_areas(&self) -> Vec<FaceArea> {
        self.extrusion_profiles()
            .into_iter()
            .filter_map(|p| match p {
                ExtrusionProfile::Face {
                    base,
                    face,
                    boundaries,
                } => Some(FaceArea {
                    body: base,
                    face,
                    boundaries,
                }),
                _ => None,
            })
            .collect()
    }
    pub(in crate::desktop::project_editor) fn capture_profile_plane(
        &mut self,
        plane: nator_sketch::Plane,
    ) {
        if self.extrusion_plane.is_none() {
            self.extrusion_plane = Some(plane);
        }
    }
}
