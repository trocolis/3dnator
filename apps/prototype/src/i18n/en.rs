//! Catálogo em inglês, na mesma ordem e com o mesmo `match` exaustivo do português:
//! um texto novo sem tradução não compila.
use super::Text;

pub fn text(key: Text) -> &'static str {
    match key {
        Text::ViewHome => "Home: isometric view and fit model",
        Text::ArrowInvalidValue => "Enter a valid measurement for this tool.",
        Text::Label => "Label",
        Text::LabelContent => "Text",
        Text::LabelFont => "System font",
        Text::LabelSize => "Size",
        Text::LabelBold => "Bold",
        Text::LabelItalic => "Italic",
        Text::LabelUnderline => "Underline",
        Text::LabelMode => "Label type",
        Text::LabelRaised => "Raised",
        Text::LabelEngraved => "Engraved",
        Text::LabelHint => {
            "Select a surface and drag the marker to position the text. The label follows the curvature; STEP keeps geometry and colors."
        }
        Text::Paint => "Paint",
        Text::PaintColor => "RGB color",
        Text::PaintSurfaces => "Selected surfaces",
        Text::PaintWholeBody => "All surfaces",
        Text::PaintCustom => "Custom",
        Text::Surface => "Surface",
        Text::PaintPickHint => {
            "Click to add or remove surfaces. Double-click selects the whole body. Visible sketches bound the region automatically."
        }
        Text::PaintInvalidHex => "Enter a color in #RRGGBB format.",
        Text::ProjectionVertex => "Vertex",
        Text::ProjectionEdge => "Edge",
        Text::ProjectionArea => "Area",
        Text::ProjectionSelected => "Selected elements",
        Text::ProjectionPickHint => {
            "Click the source or the projection to toggle the selection. Nothing is created until Finish."
        }
        Text::ProjectionFinish => "Finish",
        Text::ProjectionNothingNew => {
            "Nothing new to project: these lines are already in the sketch, as face lines (yellow) or projections."
        }
        Text::SketchImage => "Image",
        Text::ImagesFolder => "Images",
        Text::ImageDrawHint => {
            "Pick two corners on the drawing. Then adjust the properties and confirm with OK."
        }
        Text::ImageTransparency => "Transparency (%)",
        Text::ImagePosition => "Position in the part",
        Text::ImageLoad => "Load PNG or JPEG image",
        Text::ImageLocalPosition => "Position on the sketch plane",
        Text::ImageFileTooLarge => "Use an image of up to 32 MiB.",
        Text::ChooseSketchPlane => {
            "Select an origin plane, a work plane or a planar face of the model"
        }
        Text::AppName => "3Dnator",
        Text::ChooseOffsetBase => {
            "Select the plane to offset: origin plane, work plane or planar face"
        }
        Text::OffsetPlane => "Offset plane",
        Text::WorkPlanesFolder => "Planes",
        Text::WorkPlaneName => "Plane",
        Text::FlipOffset => "Flip direction",
        Text::NewSketchOnPlane => "New sketch on this plane",
        Text::WorkPlaneOtherPart => "The chosen plane belongs to another part.",
        Text::OffsetPlaneHint => "Drag the arrow or type the distance; OK creates the plane",
        Text::WorkPlaneCreated => "created. Choose it in New sketch to draw on it.",
        Text::CurrentBase => "Current base plane",
        Text::EditOffsetPlaneHint => {
            "Drag the arrow or type the distance; OK moves the plane and its sketches"
        }
        Text::PointLineDistance => "Point–line distance",
        Text::LineCircleDistance => "Line–circle distance",
        Text::DimensionPickHint => {
            "Click a line, circle, arc or point; type the value and confirm with Enter"
        }
        Text::DimensionReferenceReadOnly => {
            "This is a reference dimension: it follows the geometry. To drive a measurement, change its option in the dimensions panel."
        }
        Text::SketchMirrorAxis => "Mirror axis",
        Text::SketchMirrorAxisHint => "Pick the line in the Mirror axis field.",
        Text::SketchOffsetDouble => "Double",
        Text::SketchOffsetMode => "Offset mode",
        Text::SketchOffsetSecond => "Second side distance",
        Text::SketchOffsetPerSide => "The distance entered applies equally to each side.",
        Text::SketchChamfer => "Chamfer",
        Text::SketchWholeChain => "Connected contour",
        Text::SketchOffsetMouseHint => {
            "Drag the arrow tip to adjust the value · Enter/OK: apply · Esc: cancel"
        }
        Text::SketchDragHint => "Drag an edge or vertex to move the geometry.",
        Text::SketchCornersHint => {
            "Select the segments or a vertex; all corners will be transformed."
        }
        Text::SketchPreviewPreparing => "Preparing preview…",
        Text::SketchOffsetSelectHint => "Select an edge or a connected contour.",
        Text::SketchProject => "Project",
        Text::Restrict => "Constrain",
        Text::RestrictPickHint => "Select vertices or edges to see the possible constraints.",
        Text::RestrictArmedHint => "Select the elements of the constraint; Esc cancels.",
        Text::OriginCoincidence => "Coincident with origin",
        Text::EqualRestriction => "Equal",
        Text::Collinear => "Collinear",
        Text::Symmetry => "Symmetry",
        Text::RegularPolygon => "Regular polygon",
        Text::FixUnfix => "Fix/Release",
        Text::NormalReference => "Normal/Reference",
        Text::LinkVertex => "Link vertex",
        Text::DetachVertex => "Detach vertex",
        Text::RemoveVertex => "Remove vertex",
        Text::MergeAreas => "Merge areas",
        Text::UnlinkedEnds => "Unlinked ends. Click to link.",
        Text::FixElement => "Fix element",
        Text::ReleaseElement => "Release element",
        Text::JunctionToggleHint => {
            "Double-click the vertex or press F on the selection to fix or release the element"
        }
        Text::PointOnCurve => "Point on curve",
        Text::SketchProjectHint => {
            "Click a planar face, edge, arc or vertex of the component; Esc ends"
        }
        Text::SketchProjectAll => "All elements",
        Text::SketchProjectFilter => "Filter",
        Text::SketchProjectPlanes => "Faces and areas",
        Text::SketchProjectEdges => "Edges and arcs",
        Text::SketchProjectVertices => "Vertices",
        Text::DimensionPickSecond => "Click the second point or a line; Esc cancels the dimension",
        Text::DimensionUnsupported => "This combination does not define a dimension.",
        Text::DimensionValueTitle => "Dimension value",
        Text::DimensionInvalidValue => {
            "Invalid value: non-negative lengths, positive radii, angles between 0° and 180°."
        }
        Text::SketchHiddenPart => "Show an occurrence of the part to edit its sketch.",
        Text::Origin => "Origin",
        Text::SketchXY => "XY sketch",
        Text::ViewRight => "Right",
        Text::ViewFront => "Front",
        Text::ViewLeft => "Left",
        Text::ViewBack => "Back",
        Text::ViewBottom => "Bottom",
        Text::ViewRotateLeft => "Rotate 45° counterclockwise about the dominant face normal",
        Text::ViewRotateRight => "Rotate 45° clockwise about the dominant face normal",
        Text::ViewRollLeft => "Roll 45° left",
        Text::ViewRollRight => "Roll 45° right",
        Text::ViewCubeHint => {
            "Orientation cube: click faces, edges or corners; drag to orbit. Right-click opens the view options."
        }
        Text::MinimizeWindow => "Minimize window",
        Text::MaximizeWindow => "Maximize window",
        Text::RestoreWindow => "Restore window",
        Text::CloseWindow => "Close application",
        Text::AuxiliaryProject => "Auxiliary project",
        Text::FilletShort => "Fillet",
        Text::MirrorShort => "Mirror",
        Text::ArcShort => "Arc",
        Text::Visibility => "Show / hide",
        Text::ModelTab => "Construction",
        Text::PrintTab => "Printing",
        Text::Calibrate => "Calibrate",
        Text::CalibrateStepPrint => {
            "1. Print the part with your usual settings. The cut corner is on the side of the larger ones."
        }
        Text::CalibrateExport => "Export calibration part…",
        Text::CalibrateStepHoles => "2. Measure each hole, from left to right:",
        Text::CalibrateStepFits => {
            "3. Test the pin in the square sockets and choose the clearance of each fit:"
        }
        Text::CalibrateHole => "Hole of",
        Text::CalibrateApply => "Apply to profile",
        Text::HolePresetField => "Fastening",
        Text::HolePresetChoose => "Fill in…",
        Text::PresetThrough => "Through",
        Text::PresetRecessed => "Recessed head",
        Text::PresetSelfTap => "Self-tapping",
        Text::PresetInsert => "Heat-set insert",
        Text::PresetNut => "Captive nut",
        Text::CaptiveNutHead => "Nut",
        Text::NutFlats => "Across flats",
        Text::NutDepth => "Height",
        Text::TeardropHole => "Teardrop hole",
        Text::TeardropHint => "45° tip pointing up: horizontal holes print without support",
        Text::Fasteners => "Fasteners",
        Text::FastenerAdd => "New",
        Text::FastenerNew => "New",
        Text::FastenerDiameter => "Diameter",
        Text::FastenerPilot => "Self-tapping",
        Text::FastenerInsertDiameter => "Insert, diameter",
        Text::FastenerInsertLength => "Insert, length",
        Text::FastenerNutFlats => "Nut, across flats",
        Text::FastenerNutHeight => "Nut, height",
        Text::FastenerHeadDiameter => "Head, diameter",
        Text::FastenerHeadHeight => "Head, height",
        Text::PrintCheck => "Check print",
        Text::PrintCheckShort => "Check",
        Text::PrintCheckHint => {
            "Estimates from the geometry and the profile. Click a region to go to it."
        }
        Text::PrintCheckClean => "No problems found.",
        Text::PrintNoFace => "No face on the bed: evaluated in the modeling orientation.",
        Text::PrintContact => "Support",
        Text::PrintSmallBase => "Small support: the part may come off the bed.",
        Text::PrintRegions => "region(s)",
        Text::IssueOverhang => "Overhangs",
        Text::IssueThinWall => "Thin walls",
        Text::IssueGap => "Gaps",
        Text::BedFace => "Face on bed",
        Text::BedFaceShort => "Face on bed",
        Text::BedFaceField => "Resting face",
        Text::BedFaceHint => {
            "Click a planar face. The choice applies to printing and does not change the model."
        }
        Text::BedSize => "On the bed",
        Text::BedTooSmall => "Larger than the print profile's bed.",
        Text::BedRemove => "Back to the modeling orientation",
        Text::ExportForPrint => "Export for printing",
        Text::ExportForPrintShort => "Export",
        Text::ExportFormat => "Format",
        Text::ExportPrepareBed => "Prepare for the bed",
        Text::ExportOpenSlicer => "Open in slicer",
        Text::ExportSlicerCommand => "Command",
        Text::ExportSlicerHint => {
            "Empty uses the system's default application. {arquivo} marks where the file goes."
        }
        Text::ExportNotOpened => "Exported, but the slicer did not open",
        Text::ExportOversized => "Larger than the bed",
        Text::ExportOverflow => "The set takes more than one bed",
        Text::ExportSkipped => "Open surfaces, left out of the export",
        Text::ThreeMfFilter => "3MF model",
        Text::PrintProfile => "Print profile",
        Text::PrintProfileName => "Name",
        Text::PrintProfileLibrary => "Library",
        Text::PrintProfileChoose => "Load",
        Text::PrintProfileSave => "Save",
        Text::PrintProfileNoLibrary => "Preferences folder unavailable.",
        Text::PrintProfileHint => {
            "Initial values are rules of thumb. Adjust them with measurements from your printer."
        }
        Text::PrintProfileNames => "In expressions",
        Text::PrintNozzle => "Nozzle",
        Text::PrintLayer => "Layer",
        Text::PrintFitTight => "Tight clearance",
        Text::PrintFitSliding => "Sliding clearance",
        Text::PrintFitFree => "Free clearance",
        Text::PrintHoleCompensation => "Hole compensation",
        Text::PrintOverhang => "Maximum overhang",
        Text::PrintDensity => "Density",
        Text::ComponentCut => "Cut component",
        Text::ComponentCutShort => "Cut",
        Text::InsertionField => "Enters through",
        Text::FitClearance => "Clearance",
        Text::FloorClearance => "Bottom clearance",
        Text::ComponentDepth => "Length",
        Text::ComponentHint => "Click the component in the 3D view. It may be in another part.",
        Text::ComponentFrozen => {
            "Silhouette captured. Changing the clearance recalculates; moving the component needs a new capture."
        }
        Text::Lip => "Lip",
        Text::LipFace => "Edge face",
        Text::LipKindField => "Type",
        Text::LipRaised => "Adds",
        Text::LipGroove => "Recesses",
        Text::LipReferenceField => "Outline",
        Text::LipInner => "Inner",
        Text::LipOuter => "Outer",
        Text::LipInset => "Inset",
        Text::LipHint => {
            "Use Adds on one body and Recesses on the other, with the same measurements: the clearance goes only into the recess."
        }
        Text::FastenerHeight => "Height",
        Text::ScrewBoss => "Screw boss",
        Text::ScrewBossShort => "Boss",
        Text::BossOuter => "Outer diameter",
        Text::BossShallowHole => "Hole shallower than the boss",
        Text::BaseFillet => "Base fillet",
        Text::SnapFit => "Snap fit",
        Text::SnapHook => "Snap hook",
        Text::SnapWindow => "Window",
        Text::SnapLength => "Length",
        Text::SnapThickness => "Thickness",
        Text::SnapWidth => "Width",
        Text::SnapOverhang => "Overhang",
        Text::SnapHead => "Ramp",
        Text::SnapWindowDepth => "Beyond the head",
        Text::SnapStrain => "Estimated strain",
        Text::SnapStrainHigh => "Above the profile limit: lengthen the arm or reduce the overhang.",
        Text::FitCheck => "Check fit",
        Text::FitCheckShort => "Fit",
        Text::FitChecking => "Comparing the bodies…",
        Text::FitInterference => "Interference",
        Text::FitTouching => "Touching",
        Text::FitTight => "Small clearance",
        Text::FitClean => "No interference.",
        Text::FitCompared => "Pairs compared",
        Text::FitBeyond => "Pairs not compared, time ran out",
        Text::FitSkipped => "No volume to compare",
        Text::FitFailed => "Comparison failed",
        Text::FitAgain => "Check again",
        Text::FitStale => "The assembly changed.",
        Text::FitScopeField => "Compare",
        Text::FitScopePart => "Part being edited",
        Text::FitScopeAll => "Components",
        Text::ExcerptToggle => "Only a section",
        Text::ExcerptHint => "Box in part coordinates. The model does not change.",
        Text::PrintMass => "Estimated mass",
        Text::PrintCost => "Estimated cost",
        Text::PrintPrice => "Price per kg",
        Text::PrintInfill => "Infill",
        Text::PrintWallLines => "Wall loops",
        Text::PrintMaxStrain => "Maximum strain",
        Text::PrintEstimateHint => "Estimate before slicing, without supports.",
        Text::PrintBedLimit => "Limit to the bed size",
        Text::SettingsMenu => "Settings",
        Text::SettingsAppearance => "Appearance",
        Text::SettingsDrawing => "Drawing",
        Text::SettingsShortcuts => "Shortcuts",
        Text::SettingsAbout => "About",
        Text::SettingsAboutDescription => {
            "Parametric CAD in Rust for modeling precise mechanical parts for prototypes."
        }
        Text::SettingsAboutVersion => "Version",
        Text::SettingsAboutAuthor => "Author",
        Text::SettingsAboutAuthorValue => "Bruno Trócolis",
        Text::SettingsAboutLicense => "License",
        Text::SettingsAboutLicenseValue => "MIT",
        Text::SettingsAboutRepository => "Repository",
        Text::SettingsAboutRepositoryLink => "github.com/trocolis/3dnator",
        Text::SettingsAboutProjectFormat => "Project format",
        Text::SettingsAboutTechnologies => "Technologies",
        Text::SettingsAboutTechnologiesValue => "Rust · egui/wgpu · Open CASCADE Technology",
        Text::SettingsAboutCopyright => "© 2026 Bruno Trócolis",
        Text::SettingsUnit => "Unit of measurement",
        Text::SettingsUnitMillimeter => "Millimeter (mm)",
        Text::SettingsUnitInch => "Inch (in)",
        Text::SettingsUnitHint => {
            "Measurements and numbers typed without a unit use this scale; the geometry stays in millimeters."
        }
        Text::SettingsMouseStep => "Mouse precision",
        Text::SettingsMouseAngleStep => "Angle precision",
        Text::SettingsMouseStepHint => {
            "When drawing, moving, rotating or extruding, the mouse moves in multiples of the precision. Type a value to use any number; in the sketch, Alt frees the cursor."
        }
        Text::ThroughShell => "Go through (create duct)",
        Text::CuttingPlane => "Splitting plane",
        Text::RevolveAxisHint => {
            "Click a line of a sketch coplanar with the areas, including a reference or infinite line."
        }
        Text::CoplanarProfilesRequired => "Select areas and axis on the same plane.",
        Text::Loft => "Loft between profiles",
        Text::Pipe => "Pipe",
        Text::PipeDiameter => "Outer diameter",
        Text::PipeHollow => "Hollow tube",
        Text::PipeHint => {
            "Click a sketch circle, line or arc. Open and closed paths with tangent bends are supported. Turn off Hollow tube for a solid section."
        }
        Text::Sweep => "Guided transition",
        Text::SweepHint => {
            "Automatic guide, arc about an axis on the start plane, or an open chain of lines/arcs from a sketch. Profile planes must be perpendicular to the guide at the ends."
        }
        Text::RouteField => "Path",
        Text::AutomaticRoute => "Automatic",
        Text::LongArc => "Arc on the opposite side",
        Text::EndProfile => "End area",
        Text::StartProfile => "Start area",
        Text::RuledLoft => "Straight rulings",
        Text::DevelopableLoft => "Developable surface",
        Text::DevelopableLoftHint => {
            "From a convex polygon to a circle: flat triangles and cones, without twist, that accept shell. Other profiles use the common transition."
        }
        Text::UpdateHolePairs => "Update hole matching",
        Text::LoftHint => {
            "Choose one area on each plane. Holes must match by creation order and exist in both profiles."
        }
        Text::LoftDifferentPlanes => "Choose the end area on a plane different from the start.",
        Text::CuttingPlaneHint => {
            "Choose a planar face, a work plane or an origin plane. Both sides will be kept."
        }
        Text::ExitFace => "Exit face",
        Text::AutomaticExitHint => {
            "No selection: use the first unique opposite face. Choose the exit on the same body."
        }
        Text::SketchXZ => "XZ sketch",
        Text::SketchYZ => "YZ sketch",
        Text::SearchTools => "Search tools",
        Text::SearchHint => "Type a tool…",
        Text::NoMatchingTools => "No tool found.",
        Text::ChooseActivePart => "Select a part and use Build in this part.",
        Text::DocumentGroup => "Document",
        Text::ActivityGeometry => "Create geometry",
        Text::ActivityRelation => "Edit relations",
        Text::ActivityModify => "Modify geometry",
        Text::BrowserEmpty => "Your project starts here",
        Text::StartSketch => "Create a sketch or import a model.",
        Text::SolidBodies => "Bodies",
        Text::SketchesGroup => "Sketches",
        Text::ParentPart => "Insert inside the part",
        Text::RootAssembly => "Assembly root",
        Text::Groups => "Organizational groups",
        Text::GroupMembers => "Select instances of the same level",
        Text::SolidBody => "CAD solid",
        Text::ParametricHistory => "History",
        Text::ViewOrbitReverseHint => {
            "45° step about the dominant face normal. Shift+click or right button reverses the direction."
        }
        Text::ZeroExtrusionHint => "Zero splits the face by the profile, keeping the body volume.",
        Text::Extrude => "Extrude",
        Text::Revolve => "Revolve",
        Text::Join => "Join",
        Text::Cut => "Cut",
        Text::Intersect => "Intersect",
        Text::Fillet => "Fillet",
        Text::Chamfer => "Chamfer",
        Text::Shell => "Shell",
        Text::Profile => "Profile sketch",
        Text::NoRegions => "The sketch has no usable closed regions.",
        Text::Angle => "Angle / dimension",
        Text::TopologyIndex => {
            "Edge/face index on the base body; on apply, it becomes a reference by the source faces"
        }
        Text::KeptReference => "Current reference (kept while the index is not changed)",
        Text::BodyColor => "Body color",
        Text::NeutralColor => "Restore neutral color",
        Text::Isolate => "Isolate",
        Text::ExitIsolation => "Exit isolation",
        Text::TransparentToggle => "Transparent",
        Text::NewBody => "New body",
        Text::ExportScope => "What to export",
        Text::ScopeBody => "Selected body",
        Text::ScopePart => "Bodies of the part",
        Text::ScopeDefinitions => "Each definition once",
        Text::ScopeJoined => "Whole assembly as one object",
        Text::ScopeOccurrences => "All occurrences",
        Text::ExportQuality => "Quality",
        Text::QualityDraft => "Draft",
        Text::QualityStandard => "Standard",
        Text::QualityHigh => "High",
        Text::QualityCustom => "Custom",
        Text::LinearTolerance => "Linear deviation (mm)",
        Text::AngularTolerance => "Angular deviation (°)",
        Text::ComputeSummary => "Compute summary",
        Text::ComputeSummaryHint => "Compute the summary to see files, size and dimensions.",
        Text::ExportFiles => "Export…",
        Text::ExportStep => "Export STEP…",
        Text::ExportPartStep => "Part as STEP…",
        Text::ExportAssemblyStep => "Assembly as STEP…",
        Text::FilesLabel => "object(s)",
        Text::Occurrences => "occurrences in the assembly",
        Text::FaceLabel => "Face",
        Text::EdgeLabel => "Edge",
        Text::Length => "Length",
        Text::DiameterLabel => "Diameter",
        Text::AngleLabel => "Angle to the reference",
        Text::DrawingRevision => "Revision",
        Text::DrawingDate => "Date",
        Text::SketchOnFace => "On the selected face (captured plane)",
        Text::MirroredSuffix => "mirrored",
        Text::HoleTool => "Hole",
        Text::LinearPattern => "Linear pattern",
        Text::CircularPattern => "Circular pattern",
        Text::MirrorBody => "Mirror body",
        Text::SplitBody => "Split body",
        Text::SplitShort => "Split",
        Text::CheckSolid => "Check solid",
        Text::CheckSolidShort => "Check",
        Text::CheckSolidHint => "Choose the body to check.",
        Text::CheckingSolid => "Analyzing the body…",
        Text::SolidHasNoDefects => "No defects found: the body can be exported.",
        Text::SolidMeshDefects => {
            "This body's STL would have defects, but there are no repairable contacts. Look for microscopic gaps between nearly aligned sketches."
        }
        Text::SolidSliver => "Sliver smaller than the STL precision",
        Text::SolidSliverWidth => "Width",
        Text::SolidSliverHint => {
            "It does not change the part: the sliver leaves the mesh on export. To remove it from the model, adjust the sketch of the indicated operation."
        }
        Text::PrintSlivers => {
            "triangles smaller than the STL precision leave the mesh on export, near"
        }
        Text::Contact => "Contact",
        Text::ContactOrigin => "Appeared in",
        Text::RepairContact => "Contact repair",
        Text::RepairModeField => "Repair",
        Text::WeldContact => "Weld",
        Text::WeldContactHint => "Joins the two volumes by adding material along the contact",
        Text::SeparateContact => "Separate",
        Text::SeparateContactHint => {
            "Moves the two volumes apart by removing material along the contact"
        }
        Text::HolePoints => "Circle centers and isolated points of the sketch",
        Text::NoHolePoints => "The sketch has no circle centers or isolated points.",
        Text::HoleDiameter => "Diameter",
        Text::HoleDepth => "Depth",
        Text::Through => "Through",
        Text::HeadSimple => "Simple",
        Text::Counterbore => "Counterbore",
        Text::Countersink => "Countersink",
        Text::HeadDiameter => "Head diameter",
        Text::HeadDepth => "Counterbore depth",
        Text::HeadAngle => "Countersink angle",
        Text::DrillPoint => "Drill point",
        Text::ReverseDirection => "Reverse direction",
        Text::ThreadToggle => "Simplified thread (recorded, no helix)",
        Text::ThreadDesignation => "Designation",
        Text::ThreadNominal => "Nominal diameter",
        Text::ThreadPitch => "Pitch",
        Text::SecondDirection => "Second direction",
        Text::TotalAngle => "Total angle",
        Text::KeepOriginal => "Keep the original (mirrored part)",
        Text::FlipSide => "Keep the other side",
        Text::PickOtherPart => "The selected body belongs to another part.",
        Text::KernelUnavailable => {
            "OCCT SDK required: build with the occt feature to create solids."
        }
        Text::ModelFilter => "STL, STEP or 3MF models",
        Text::StepFilter => "STEP geometry",
        Text::OutputExists => "The destination already exists; choose a new name.",
        #[cfg(feature = "occt")]
        Text::MeshIsNotSolid => "The mesh has no precise CAD geometry to export as STEP.",
        Text::Tangent => "Tangency",
        Text::Drawing => "Technical drawing",
        Text::DrawingTitle => "Title",
        Text::Paper => "Paper",
        Text::Blueprint => "Blueprint style",
        Text::HiddenLines => "Hidden lines",
        Text::DrawingDimensions => "Overall view dimensions",
        Text::Preview => "Update preview",
        Text::SaveSheet => "Save sheet in the project",
        Text::ExportPdf => "Export vector PDF",
        Text::ExportPng => "Export PNG",
        Text::Section => "Section view A–A",
        Text::SectionPosition => "Cutting plane origin (mm)",
        Text::PlaneNormal => "Cutting plane normal",
        Text::AssociativeDimensions => "Associative dimensions",
        Text::DimensionKind => "Dimension type",
        Text::DimensionView => "Dimension view",
        Text::DimensionLength => "Edge length",
        Text::DimensionRadiusFace => "Face radius",
        Text::DimensionDiameterFace => "Face diameter",
        Text::DimensionRadiusEdge => "Circular edge radius",
        Text::DimensionDiameterEdge => "Circular edge diameter",
        Text::DimensionFaceDistance => "Distance between parallel faces",
        Text::DimensionFaceAngle => "Angle between planar faces",
        Text::CaptureFirstReference => "Capture first selected reference",
        Text::CaptureSecondReference => "Capture second selected reference",
        Text::AddDimension => "Add dimension",
        Text::RemoveDimension => "Remove dimension",
        Text::UpdateDimensionReferences => "Apply references to the chosen dimension",
        Text::DimensionTextPosition => {
            "Text offset on paper (mm); you can also drag it in the preview"
        }
        Text::SupplementaryAngle => "Supplementary angle",
        Text::DimensionReview => "Dimensions that need review",
        Text::StaleDrawingPreview => {
            "Preview from before the change. Update it before repositioning the dimensions."
        }
        Text::DrawingReferenceMissing => "Reference removed",
        Text::AddSection => "Add section",
        Text::RemoveSection => "Remove section",
        Text::Portrait => "Portrait",
        Text::PngResolution => "PNG resolution (DPI)",
        Text::MeshDrawingNotice => {
            "The legend identifies views from the B-rep or approximated from the mesh. Mesh sections are approximate; overall dimensions measure the projected limits."
        }
        Text::Title => "3Dnator · Projects",
        Text::NewProject => "New",
        Text::OpenProject => "Open project",
        Text::Save => "Save",
        Text::SaveAs => "Save as…",
        Text::RecentProjects => "Recent projects",
        Text::Undo => "Undo",
        Text::Redo => "Redo",
        Text::Import => "Import model",
        Text::Reference => "Insert L bracket",
        Text::Project => "Project tree",
        Text::Untitled => "Untitled",
        Text::MeshBody => "Mesh",
        Text::Triangles => "Triangles",
        Text::Triangulation => "Show triangulation",
        Text::Shadows => "Shadows",
        Text::Projection => "Projection",
        Text::Orthographic => "Orthographic",
        Text::Perspective => "Perspective",
        Text::Fit => "Fit",
        Text::Front => "Front",
        Text::Top => "Top",
        Text::Side => "Side",
        Text::Iso => "Isometric",
        Text::Loading => "Processing…",
        Text::Ready => "Ready",
        Text::Navigation => "Scroll: zoom · Middle: pan · Shift + middle: orbit",
        Text::Scale => "STL scale to millimeters",
        Text::Failed => "The operation failed",
        Text::Export => "Export STL…",
        Text::Exported => "Export finished",
        Text::Saved => "Project saved",
        Text::Imported => "Body imported",
        Text::Assembly => "Assembly",
        Text::Construction => "Reference",
        Text::EditPart => "Build in this part",
        Text::Position => "Position X / Y / Z (mm)",
        Text::Close => "Close",
        Text::Unsaved => "Unsaved changes",
        Text::SaveChanges => "Save changes before closing?",
        Text::Discard => "Discard",
        Text::Cancel => "Cancel",
        Text::Recovery => "Recovery",
        Text::Recover => "Recover",
        Text::RecoverySaved => "Automatic recovery updated",
        Text::RecoveryFailed => "Could not update the recovery",
        Text::EmptyScene => "Create a sketch or import a model to start.",
        Text::FileFilter => "3Dnator project",
        Text::StlFilter => "STL body",
        Text::RecoveryNotice => {
            "There are recovery copies. Open one to review and save the project."
        }
        Text::FileAlreadyOpen => "This file is already open in another tab.",
        Text::StaleResult => "Result ignored: the project changed during the operation.",
        Text::Sketch => "Sketch",
        Text::AutomaticExtrusion => "Automatic operation",
        Text::ExtrusionProfileHint => {
            "Click an area: sketch region, part of the face bounded by it, or a planar face. Click again to remove. Another plane or face becomes the limit."
        }
        Text::ExtrusionMouseHint => {
            "The extrusion starts at 0 mm. Drag the arrow or type the distance. Click another plane to end on it. Choosing an operation turns off the automatic suggestion."
        }
        Text::RemoveMaterial => "Remove",
        Text::FindingClosedAreas => "Finding closed areas…",
        Text::ProfileQueryFailed => "Could not identify the closed areas.",
        Text::SketchSweep => "Sweep",
        Text::SketchDynamicWidth => "Width",
        Text::SketchDynamicHeight => "Height",
        Text::SketchValueLocked => {
            "Value locked by typing. Clear the field to follow the cursor again."
        }
        Text::SketchDynamicHint => "Tab: switch · Enter: confirm · Esc: cancel",
        Text::SketchLineMode => "Line type",
        Text::SketchRectangleMode => "Rectangle type",
        Text::SketchCircleMode => "Circle type",
        Text::SketchRectangleCenter => "From the center out",
        Text::SketchRectangleInclined => "Inclined",
        Text::SketchCircleDiameter => "Two points (diameter)",
        Text::SketchCircleTangent => "Tangent",
        Text::SketchContinuousLineHint => {
            "Click the next vertices. Esc ends the sequence; closing an area or ending on another element also ends it. Starting on a circle or arc, hold Shift to slide the start along it with the line tangent."
        }
        Text::SketchSymmetricLineHint => {
            "The first point is the center: each segment creates another of the same size on the opposite side."
        }
        Text::SketchRectangleNormalHint => "Click two opposite corners.",
        Text::SketchRectangleCenterHint => {
            "Click the center and then a corner. The values show total width and height."
        }
        Text::SketchRectangleInclinedHint => {
            "Click the start and end of one side; then set the perpendicular width."
        }
        Text::SketchCircleNormalHint => "Click the center and then the edge to set the radius.",
        Text::SketchCircleDiameterHint => "Click both ends of the diameter.",
        Text::SketchCircleTangentHint => {
            "Select two perpendicular lines. Move the mouse to choose the quadrant and radius, or type the radius; click or Enter confirms. Uses the supporting lines."
        }
        Text::SketchPickFirstTangent => "Select the first supporting line.",
        Text::SketchPickSecondTangent => "Select the second line, perpendicular to the first.",
        Text::SketchPickLine => "Select a line of the sketch.",
        Text::SketchLineType => "Stroke",
        Text::SketchLineNormal => "Normal",
        Text::SketchLineInfinite => "Infinite",
        Text::SketchLineNormalHint => "Drawing geometry; takes part in areas and extrusions.",
        Text::SketchLineConstructionHint => {
            "Orange dashed reference geometry; does not define areas or extrusions."
        }
        Text::SketchLineInfiniteHint => "Orange dashed line without ends; for reference only.",
        Text::SketchRelativeAngleHint => {
            "Angle relative to the line captured at the first point; positive counterclockwise."
        }
        Text::SketchAngleValue => "Angle (°)",
        Text::SketchHorizontalAngleHint => {
            "Angle relative to the sketch's horizontal axis; positive counterclockwise."
        }
        Text::SketchDimension => "Dimension",
        Text::HorizontalVertical => "Horizontal/vertical",
        Text::SketchExtendShort => "Extend",
        Text::SketchValue => "Value (mm)",
        Text::SketchEntities => "Entities",
        Text::SketchSelectOne => "Select an entity in the sketch.",
        Text::SketchSelectTwo => "Select two entities in the sketch, in the order shown.",
        Text::SketchSelectDimension => "Select a dimension or a line, circle or arc.",
        Text::SketchMirrorSelectionHint => {
            "Select the elements in the Entities field and choose a line in the Mirror axis field."
        }
        Text::SketchTangentSelectionHint => "Select a line and an arc with a shared end.",
        Text::SketchCoincidentSelectionHint => {
            "Select two entities. Their nearest ends (or circle centers) will be made coincident."
        }
        Text::Parameters => "Parameters",
        Text::ParameterName => "Name",
        Text::Expression => "Expression",
        Text::CurrentValue => "Current value",
        Text::LengthKind => "Length",
        Text::AngleKind => "Angle",
        Text::ScalarKind => "Unitless",
        Text::ExpressionHint => "Examples: width / 2; 10 mm; 1 in. Use explicit units in sums.",
        Text::ApplyParameter => "Apply parameter",
        Text::DeleteParameter => "Delete parameter",
        Text::Fixed => "Fixed point",
        Text::Coincident => "Coincidence",
        Text::Horizontal => "Horizontal",
        Text::Vertical => "Vertical",
        Text::HorizontalDimension => "Horizontal distance",
        Text::VerticalDimension => "Vertical distance",
        Text::Radius => "Radius",
        Text::Midpoint => "Midpoint",
        Text::PointOnLine => "Point on line",
        Text::Concentric => "Concentricity",
        Text::EditDimension => "Change selected dimension",
        Text::NewSketch => "New sketch",
        Text::SketchConflict => "Conflicting constraints — change pending",
        Text::FullyConstrained => "Fully constrained",
        Text::UnderConstrained => "Free or partially constrained",
        Text::DegreesOfFreedom => "Degrees of freedom",
        Text::Redundant => "There are redundant constraints",
        Text::Select => "Select",
        Text::Line => "Line",
        Text::Rectangle => "Rectangle",
        Text::Circle => "Circle",
        Text::Arc => "Arc: center, start and end",
        Text::Trim => "Trim",
        Text::Dimension => "Dimension value / offset",
        Text::ReferenceDimension => "Reference dimension",
        Text::Entity => "Entity",
        Text::FixPoint => "Fix first point / center",
        Text::ConstructionGeometry => "Toggle Normal/Reference stroke",
        Text::DeleteEntity => "Delete selected entities",
        Text::Parallel => "Parallel",
        Text::Perpendicular => "Perpendicular",
        Text::EqualLength => "Equal lengths",
        Text::Constraints => "Dimensions and constraints",
        Text::FinishSketch => "Finish sketch",
        Text::FinishSketchShort => "Finish",
        Text::CancelSketch => "Cancel sketch",
        Text::InvalidArc => "Invalid arc: choose a distinct center and ends.",
        Text::PendingEdit => "Finish or cancel the pending change before saving.",
        Text::TimelineFirst => "Go to the start of the history",
        Text::TimelinePrevious => "Step back one operation",
        Text::TimelineNext => "Step forward one operation",
        Text::TimelineLast => "Go to the end of the history",
        Text::HistoryMarker => "History marker: drag to go back to a step",
        Text::EditFeature => "Edit operation",
        Text::EditProfileSketch => "Edit profile sketch",
        Text::RenameFeature => "Rename",
        Text::SuppressFeature => "Suppress",
        Text::UnsuppressFeature => "Reactivate",
        Text::MarkerHere => "Move marker here",
        Text::DeleteFeature => "Delete",
        Text::StateActive => "Active",
        Text::StateSuppressed => "Suppressed",
        Text::StateDependent => "Inactive: depends on a suppressed operation",
        Text::StateRolledBack => "After the history marker",
        Text::StateFailed => "Failed when recalculating the pending edit",
        Text::StateAffected => "Depends on the failed operation",
        Text::DeleteFeatureTitle => "Delete operation",
        Text::DeleteDependents => "These operations depend on the result and will also be deleted:",
        Text::DeleteAll => "Delete all",
        Text::RebuildFailedTitle => "Recalculation failed",
        Text::RebuildFailedAffected => "Affected operations:",
        Text::FixFeature => "Edit the operation",
        Text::SuppressAndApply => "Suppress and apply",
        Text::CancelEdit => "Cancel edit",
        Text::PendingEditBanner => "Pending edit: showing the last valid result, from before it.",
        Text::FixingFailure => {
            "Fixing the failed operation; on apply, the pending edit goes in together."
        }
        Text::RenameFeatureTitle => "Rename operation",
        Text::TimelineDragHint => "Drag to reorder; right-click for more options.",
        Text::Ok => "OK",
        Text::PreviewComputing => "Computing preview…",
        Text::PreviewReady => "Preview updated",
        Text::PreviewFailed => "Could not compute",
        Text::SelectPrompt => "Select",
        Text::SelectedCount => "selected",
        Text::ClearSelection => "Clear selection",
        Text::ProfileField => "Profile",
        Text::ProfileHint => "Click a closed region of the sketch in the 3D view.",
        Text::DirectionField => "Direction",
        Text::OneSide => "One side",
        Text::TwoSides => "Two sides",
        Text::Symmetric => "Symmetric",
        Text::ExtentField => "Extent",
        Text::ExtentDistance => "Distance",
        Text::ExtentToFace => "Up to a plane",
        Text::DistanceField => "Distance",
        Text::Distance2Field => "Distance 2",
        Text::StartOffset => "Start offset",
        Text::FlipDirection => "Flip direction",
        Text::OperationField => "Operation",
        Text::TargetBody => "Target body",
        Text::EdgesField => "Edges",
        Text::FacesField => "Faces",
        Text::RadiusField => "Radius",
        Text::ThicknessField => "Thickness",
        Text::ChamferType => "Type",
        Text::EqualDistance => "Equal distance",
        Text::DistanceAngle => "Distance and angle",
        Text::InwardWalls => "Inward",
        Text::OutwardWalls => "Outward",
        Text::AxisField => "Axis",
        Text::FullTurn => "Full turn",
        Text::BodyField => "Body",
        Text::ToolBodyField => "Tool body",
        Text::CountField => "Count",
        Text::SpacingField => "Spacing",
        Text::PlaneField => "Plane",
        Text::OffsetField => "Offset",
        Text::OriginField => "Origin",
        Text::AdvancedGroup => "Advanced",
        Text::FromField => "Start",
        Text::ToField => "End",
        Text::HoleType => "Hole type",
        Text::HoleExtent => "Extent",
        Text::CombineTool => "Combine",
        Text::RectangularPattern => "Rectangular pattern",
        Text::SolidBodyRequired => {
            "Select a CAD solid; mesh bodies do not take part in operations."
        }
        Text::SelectOtherTitle => "Select among overlapping",
        Text::CopySuffix => "copy",
        Text::GroupLabel => "Group",
        Text::MoveCopyTool => "Move/Copy",
        Text::AlignTool => "Align",
        Text::CircularInstancePattern => "Circular component pattern",
        Text::RectangularInstancePattern => "Rectangular component pattern",
        Text::MirrorComponent => "Mirror component",
        Text::ObjectsField => "Objects",
        Text::TranslationField => "Translation",
        Text::RotationField => "Rotation",
        Text::PivotField => "Pivot",
        Text::PivotCenter => "Center of mass",
        Text::PivotOrigin => "Origin of the first element",
        Text::CreateCopy => "Create copy (new instance or body)",
        Text::ResetTransform => "Reset",
        Text::MoveHint => {
            "Drag the arrows to translate, the rings to rotate and the center to move freely."
        }
        Text::InvalidTransformNumber => {
            "Fix the highlighted fields. Use numbers or expressions between −10,000,000 and 10,000,000 in the unit shown; Esc restores the last valid value."
        }
        Text::AlignFrom => "Source",
        Text::AlignTo => "Target",
        Text::AlignHint => {
            "Click the geometry of the moving element and then the target, in another component or in a reference sketch."
        }
        Text::ComponentField => "Component",
        Text::MirrorHint => {
            "Creates a new, independent part, reflected by the plane in parent coordinates."
        }
        Text::CloneTool => "Independent clone",
        Text::GroupTool => "Group",
        Text::UngroupTool => "Ungroup",
        Text::Selectable => "Selectable",
        Text::Hide => "Hide",
        Text::Show => "Show",
        Text::RenameDefinition => "Rename part definition",
        Text::SelectMembers => "Select members",
        Text::StepShort => "STEP",
        Text::SectionAnalysis => "Section cut",
        Text::PickedFace => "Chosen face",
        Text::SectionFaceHint => "Uses the plane of the face chosen in the 3D view, captured now.",
        Text::SectionHint => {
            "Drag the arrow to move the plane. The cut is visual only: it does not change the model or the undo history, and selection ignores the cut part."
        }
        Text::AnalysesGroup => "Analyses",
        Text::EditFeatureShort => "Edit",
        Text::RepairSketch => "Repair sketch",
        Text::RepairTolerance => "Maximum gap",
        Text::RepairDuplicate => "Remove duplicate entity",
        Text::RepairContained => "Remove overlapping segment",
        Text::RepairGap => "Join open ends",
        Text::RepairLinkEnds => "Link ends at the same place",
        Text::RepairHorizontal => "Make horizontal",
        Text::RepairVertical => "Make vertical",
        Text::RepairCrossing => "Crossing at",
        Text::RepairCrossingHint => "use Trim to choose the segment",
        Text::RepairNothing => "No fixable problem found.",
        Text::RepairHint => {
            "Checked fixes appear in orange in the sketch and are only applied with OK. Joining ends adds a coincidence solved by the solver."
        }
        Text::DisplayDecimals => "Displayed decimal places",
        Text::DisplayDecimalsHint => {
            "Affects measurements, properties and parameters in the interface; internal precision does not change."
        }
        Text::InsertFromProject => "Part from another project",
        Text::NoOtherProjects => "Open another project in a tab to insert its parts.",
        Text::SourceProject => "Source project",
        Text::PartsField => "Available parts",
        Text::SubpartsCount => "subparts",
        Text::InsertIntoPart => "The copy goes in as a subpart of",
        Text::InsertAtRoot => {
            "The copy goes in at the assembly root, independent of the source file."
        }
        Text::SketchFaceShort => "On face",
        Text::PivotPicked => "Point chosen in the 3D view (vertex, edge or face)",
        Text::InsertMesh => "Insert STL mesh",
        Text::FileUnit => "File unit",
        Text::CustomScale => "Factor",
        Text::FileSize => "Dimensions read",
        Text::ResultSize => "Dimensions in mm",
        Text::MeshImportHint => {
            "STL does not state its unit. Check the dimensions before inserting; the mesh goes in as a reference body, without conversion to a solid."
        }
        Text::ReferenceSketch => "Reference sketch",
        Text::NewPartMenu => "New part",
        Text::AddFaceLines => "Bring in face lines",
        Text::ShortcutsSystem => "System",
        Text::ShortcutsWorkspace => "Construction and assembly",
        Text::ShortcutsSketch => "Sketch",
        Text::ShortcutsModel => "Construction",
        Text::ShortcutsAssembly => "Assembly",
        Text::ShortcutStroke => "Toggle reference stroke",
        Text::ShortcutConflict => "That key is already used in this context",
        Text::ShortcutsHint => {
            "Click an action's key and press the new combination; Esc gives up. The same letter can work in different contexts."
        }
        Text::ShortcutWaiting => "Press the key…",
        Text::ShortcutsReset => "Restore defaults",
        Text::SettingsTheme => "Theme",
        Text::SettingsSystem => "System",
        Text::SettingsThemeLight => "Light",
        Text::SettingsThemeDark => "Dark",
        Text::SettingsLanguage => "Language",
        Text::SettingsConfigure => "Configure",
        Text::SettingsShortcutsHint => "Keys that trigger the tools, grouped by context.",
        Text::SettingsThemeHint => {
            "Applies to all projects on this computer. System follows the operating system's theme."
        }
        Text::SettingsLanguageHint => {
            "Applies to all projects on this computer. System follows the operating system's language; other than Portuguese, Spanish and English, Portuguese is used."
        }
        Text::SettingsAboutButton => "About 3dnator",
        Text::SketchTextX => "Position X",
        Text::SketchTextY => "Position Y",
        Text::SketchTextAdjustHint => "Adjust the properties and confirm to apply the text.",
        Text::SketchTextTool => "Text",
        Text::SketchTextHint => "Click the text's insertion point, at its center.",
        Text::SketchTextDirectionHint => {
            "Click the text's direction, or confirm with the panel's rotation."
        }
        Text::ReferencesFolder => "References",
        Text::ReferenceModelHint => {
            "Model imported as a reference: it can be moved, hidden and measured, but not edited."
        }
        Text::ConvertToReference => "Convert to reference object",
        Text::ConvertToReferenceHint => {
            "Applies to every instance of this part. Confirm or cancel the open tool before converting."
        }
        Text::ReferencesDropHint => {
            "Drag a part, subpart or reference here: it becomes a reference here, keeping its position."
        }
        Text::CreatePartFromReference => "Create part",
        Text::ReferenceNotEditable => {
            "References cannot be edited. Use Create part in its menu, in the tree."
        }
        Text::MergeCoplanarAreas => "Merge coplanar areas",
        Text::ImportMenu => "Import",
        Text::ImportReference => "Import STEP, STL or 3MF as a reference",
        Text::MovementMode => "Movement mode",
        Text::MoveMenu => "Move",
        Text::MoveFree => "Free",
        Text::MoveFaceToFace => "Face to face",
        Text::MoveEdgeToEdge => "Edge to edge",
        Text::MovePointToPoint => "Vertex to vertex",
        Text::CopyMenu => "Copy",
        Text::CopyType => "Copy type",
        Text::CopyNewInstance => "New instance",
        Text::CopyIndependent => "Independent copy",
        Text::NewPartName => "Part",
        Text::MoveFreeHint => "3D manipulator and translation and rotation values.",
        Text::MoveFaceHint => {
            "Opposite and coplanar faces; accepts the plane of a reference sketch as target."
        }
        Text::MoveEdgeHint => "Parallel edges; accepts lines of reference sketches as target.",
        Text::MovePointHint => {
            "Click a body first: only its points are mapped. Vertices, midpoints, circle and arc centers and edge quadrants: bring the cursor close to snap. Sketch points and origins work as target without choosing a body."
        }
        Text::PointBodyPrompt => "Click a body to map its points.",
        Text::PointBody => "Points of",
        Text::PointBodyChange => "Change body",
        Text::DeleteRecovery => "Delete",
        Text::DeleteRecoveryConfirm => "Delete permanently",
        Text::RecoveryDeleted => "Recovery copy deleted.",
        Text::RecoveryDeleteFailed => "Could not delete the recovery copy",
        Text::MakeIndependent => "Make independent",
        Text::MakeIndependentHint => {
            "This copy shares its definition with other instances. It gets its own definition, in the same place: editing one no longer changes the others."
        }
        Text::PickForeignReference => {
            "This reference belongs to the project or to another part. Move it in Assembly or drag it, in the tree, into this part's References."
        }
        Text::ConvertWaitWork => "Wait for the running calculation to finish and convert again.",
        Text::ConvertFinishSketch => "Finish the open sketch before converting.",
        Text::ConvertResolveFailure => "Resolve the pending history failure before converting.",
        Text::VirtualCenter => "Circular center (virtual)",
        Text::VirtualArcCenter => "Arc center (virtual)",
        Text::EdgeMidpoint => "Edge midpoint",
        Text::FaceCenter => "Face center",
        Text::CircleBoundaryPoint => "Circular edge quadrant",
        Text::PointQueryFailed => "Could not query the snap points.",
        Text::CopyInstanceHint => "Same definition, beside the original: editing one changes all.",
        Text::CopyIndependentHint => {
            "New part beside the original, with its own history and no link."
        }
        Text::CopyPatternHint => {
            "Repeated instances in a rectangular or circular pattern, gathered in a group."
        }
        Text::ReferenceSketchHint => {
            "Assembly sketch for positioning parts; it does not create bodies."
        }
        Text::LeavePart => "Back to the assembly",
        Text::ProjectNode => "Project",
        Text::CollapseTree => "Collapse the tree",
        Text::ExpandTree => "Show the tree",
        Text::MeasureTool => "Measure",
        Text::SketchDiagnostics => "Sketch diagnostics",
        Text::SketchDiagnosticsShort => "Diagnostics",
        Text::PanCamera => "Pan: move the camera with the left button",
        Text::OrbitCamera => "Orbit: rotate the camera with the left button",
        Text::PartsFolder => "Parts",
        Text::SubpartsFolder => "Subparts",
        Text::StopEditPart => "Finish editing the part",
    }
}
