//! Catálogo em espanhol, na mesma ordem e com o mesmo `match` exaustivo do português:
//! um texto novo sem tradução não compila.
use super::Text;

pub fn text(key: Text) -> &'static str {
    match key {
        Text::ViewHome => "Inicio: vista isométrica y encuadrar modelo",
        Text::ArrowInvalidValue => "Introduzca una medida válida para esta herramienta.",
        Text::Label => "Rótulo",
        Text::LabelContent => "Texto",
        Text::LabelFont => "Fuente del sistema",
        Text::LabelSize => "Tamaño",
        Text::LabelBold => "Negrita",
        Text::LabelItalic => "Cursiva",
        Text::LabelUnderline => "Subrayado",
        Text::LabelMode => "Tipo de rótulo",
        Text::LabelRaised => "Relieve",
        Text::LabelEngraved => "Grabado",
        Text::LabelHint => {
            "Seleccione una superficie y arrastre el marcador para colocar el texto. El rótulo sigue la curvatura; STEP conserva geometría y colores."
        }
        Text::Paint => "Pintar",
        Text::PaintColor => "Color RGB",
        Text::PaintSurfaces => "Superficies seleccionadas",
        Text::PaintWholeBody => "Todas las superficies",
        Text::PaintCustom => "Personalizado",
        Text::Surface => "Superficie",
        Text::PaintPickHint => {
            "Haga clic para añadir o quitar superficies. Doble clic selecciona el cuerpo entero. Los bocetos visibles delimitan la región automáticamente."
        }
        Text::PaintInvalidHex => "Introduzca un color en formato #RRGGBB.",
        Text::ProjectionVertex => "Vértice",
        Text::ProjectionEdge => "Arista",
        Text::ProjectionArea => "Área",
        Text::ProjectionSelected => "Elementos seleccionados",
        Text::ProjectionPickHint => {
            "Haga clic en el origen o en la proyección para alternar la selección. No se crea nada hasta Finalizar."
        }
        Text::ProjectionFinish => "Finalizar",
        Text::ProjectionNothingNew => {
            "Nada nuevo que proyectar: esas líneas ya están en el boceto, como líneas de la cara (amarillas) o proyecciones."
        }
        Text::SketchImage => "Imagen",
        Text::ImagesFolder => "Imágenes",
        Text::ImageDrawHint => {
            "Elija dos esquinas en el dibujo. Después ajuste las propiedades y confirme con Aceptar."
        }
        Text::ImageTransparency => "Transparencia (%)",
        Text::ImagePosition => "Posición en la pieza",
        Text::ImageLoad => "Cargar imagen PNG o JPEG",
        Text::ImageLocalPosition => "Posición en el plano del boceto",
        Text::ImageFileTooLarge => "Use una imagen de hasta 32 MiB.",
        Text::ChooseSketchPlane => {
            "Seleccione un plano del origen, un plano de trabajo o una cara plana del modelo"
        }
        Text::AppName => "3Dnator",
        Text::ChooseOffsetBase => {
            "Seleccione el plano a desplazar: plano del origen, plano de trabajo o cara plana"
        }
        Text::OffsetPlane => "Plano desplazado",
        Text::WorkPlanesFolder => "Planos",
        Text::WorkPlaneName => "Plano",
        Text::FlipOffset => "Invertir sentido",
        Text::NewSketchOnPlane => "Nuevo boceto en este plano",
        Text::WorkPlaneOtherPart => "El plano elegido pertenece a otra pieza.",
        Text::OffsetPlaneHint => "Arrastre la flecha o escriba la distancia; Aceptar crea el plano",
        Text::WorkPlaneCreated => "creado. Elíjalo en Nuevo boceto para dibujar en él.",
        Text::CurrentBase => "Plano base actual",
        Text::EditOffsetPlaneHint => {
            "Arrastre la flecha o escriba la distancia; Aceptar mueve el plano y sus bocetos"
        }
        Text::PointLineDistance => "Distancia punto–línea",
        Text::LineCircleDistance => "Distancia línea–círculo",
        Text::DimensionPickHint => {
            "Haga clic en una línea, círculo, arco o punto; escriba el valor y confirme con Enter"
        }
        Text::DimensionReferenceReadOnly => {
            "Esta cota es de referencia: sigue la geometría. Para controlar una medida, cambie su opción en el panel de cotas."
        }
        Text::SketchMirrorAxis => "Eje de simetría",
        Text::SketchMirrorAxisHint => "Elija la línea en el campo Eje de simetría.",
        Text::SketchOffsetDouble => "Doble",
        Text::SketchOffsetMode => "Modo de desplazamiento",
        Text::SketchOffsetSecond => "Distancia del segundo lado",
        Text::SketchOffsetPerSide => "La distancia indicada se aplica por igual a cada lado.",
        Text::SketchChamfer => "Chaflán",
        Text::SketchWholeChain => "Contorno conectado",
        Text::SketchOffsetMouseHint => {
            "Arrastre la punta de la flecha para ajustar la medida · Enter/Aceptar: aplicar · Esc: cancelar"
        }
        Text::SketchDragHint => "Arrastre una arista o un vértice para mover la geometría.",
        Text::SketchCornersHint => {
            "Seleccione los segmentos o un vértice; se transformarán todas las esquinas."
        }
        Text::SketchPreviewPreparing => "Preparando vista previa…",
        Text::SketchOffsetSelectHint => "Seleccione una arista o un contorno conectado.",
        Text::SketchProject => "Proyectar",
        Text::Restrict => "Restringir",
        Text::RestrictPickHint => {
            "Seleccione vértices o aristas para ver las restricciones posibles."
        }
        Text::RestrictArmedHint => "Seleccione los elementos de la restricción; Esc cancela.",
        Text::OriginCoincidence => "Coincidencia con el origen",
        Text::EqualRestriction => "Iguales",
        Text::Collinear => "Colineales",
        Text::Symmetry => "Simetría",
        Text::RegularPolygon => "Polígono regular",
        Text::FixUnfix => "Fijar/Liberar",
        Text::NormalReference => "Normal/Referencia",
        Text::LinkVertex => "Unir vértice",
        Text::DetachVertex => "Separar vértice",
        Text::RemoveVertex => "Eliminar vértice",
        Text::MergeAreas => "Unir áreas",
        Text::UnlinkedEnds => "Extremos sin unir. Haga clic para unir.",
        Text::FixElement => "Fijar elemento",
        Text::ReleaseElement => "Liberar elemento",
        Text::JunctionToggleHint => {
            "Doble clic en el vértice o F en la selección fija o libera el elemento"
        }
        Text::PointOnCurve => "Punto sobre curva",
        Text::SketchProjectHint => {
            "Haga clic en una cara plana, arista, arco o vértice del componente; Esc termina"
        }
        Text::SketchProjectAll => "Todos los elementos",
        Text::SketchProjectFilter => "Filtrar",
        Text::SketchProjectPlanes => "Caras y áreas",
        Text::SketchProjectEdges => "Aristas y arcos",
        Text::SketchProjectVertices => "Vértices",
        Text::DimensionPickSecond => {
            "Haga clic en el segundo punto o en una línea; Esc cancela la cota"
        }
        Text::DimensionUnsupported => "Esta combinación no define una cota.",
        Text::DimensionValueTitle => "Valor de la cota",
        Text::DimensionInvalidValue => {
            "Valor no válido: longitudes no negativas, radios positivos, ángulos entre 0° y 180°."
        }
        Text::SketchHiddenPart => "Muestre una instancia de la pieza para editar su boceto.",
        Text::Origin => "Origen",
        Text::SketchXY => "Boceto XY",
        Text::ViewRight => "Derecha",
        Text::ViewFront => "Frente",
        Text::ViewLeft => "Izquierda",
        Text::ViewBack => "Posterior",
        Text::ViewBottom => "Inferior",
        Text::ViewRotateLeft => {
            "Girar 45° en sentido antihorario alrededor de la normal de la cara predominante"
        }
        Text::ViewRotateRight => {
            "Girar 45° en sentido horario alrededor de la normal de la cara predominante"
        }
        Text::ViewRollLeft => "Girar 45° a la izquierda",
        Text::ViewRollRight => "Girar 45° a la derecha",
        Text::ViewCubeHint => {
            "Cubo de orientación: haga clic en caras, aristas o esquinas; arrastre para orbitar. El botón derecho abre las opciones de vista."
        }
        Text::MinimizeWindow => "Minimizar ventana",
        Text::MaximizeWindow => "Maximizar ventana",
        Text::RestoreWindow => "Restaurar ventana",
        Text::CloseWindow => "Cerrar aplicación",
        Text::AuxiliaryProject => "Proyecto auxiliar",
        Text::FilletShort => "Redondear",
        Text::MirrorShort => "Simetría",
        Text::ArcShort => "Arco",
        Text::Visibility => "Mostrar / ocultar",
        Text::ModelTab => "Construcción",
        Text::PrintTab => "Impresión",
        Text::Calibrate => "Calibrar",
        Text::CalibrateStepPrint => {
            "1. Imprima la pieza con su configuración habitual. La esquina cortada queda del lado de los mayores."
        }
        Text::CalibrateExport => "Exportar pieza de calibración…",
        Text::CalibrateStepHoles => "2. Mida cada agujero, de izquierda a derecha:",
        Text::CalibrateStepFits => {
            "3. Pruebe el pasador en los encajes cuadrados y elija la holgura de cada ajuste:"
        }
        Text::CalibrateHole => "Agujero de",
        Text::CalibrateApply => "Aplicar al perfil",
        Text::HolePresetField => "Fijación",
        Text::HolePresetChoose => "Rellenar…",
        Text::PresetThrough => "Pasante",
        Text::PresetRecessed => "Cabeza embutida",
        Text::PresetSelfTap => "Autorroscante",
        Text::PresetInsert => "Inserto térmico",
        Text::PresetNut => "Tuerca cautiva",
        Text::CaptiveNutHead => "Tuerca",
        Text::NutFlats => "Entre caras",
        Text::NutDepth => "Altura",
        Text::TeardropHole => "Agujero en gota",
        Text::TeardropHint => {
            "Punta a 45° hacia arriba: los agujeros horizontales se imprimen sin soporte"
        }
        Text::Fasteners => "Fijaciones",
        Text::FastenerAdd => "Nuevo",
        Text::FastenerNew => "Nuevo",
        Text::FastenerDiameter => "Diámetro",
        Text::FastenerPilot => "Autorroscante",
        Text::FastenerInsertDiameter => "Inserto, diámetro",
        Text::FastenerInsertLength => "Inserto, longitud",
        Text::FastenerNutFlats => "Tuerca, entre caras",
        Text::FastenerNutHeight => "Tuerca, altura",
        Text::FastenerHeadDiameter => "Cabeza, diámetro",
        Text::FastenerHeadHeight => "Cabeza, altura",
        Text::PrintCheck => "Verificar impresión",
        Text::PrintCheckShort => "Verificar",
        Text::PrintCheckHint => {
            "Estimaciones por la geometría y el perfil. Haga clic en una región para ir a ella."
        }
        Text::PrintCheckClean => "No se encontraron problemas.",
        Text::PrintNoFace => "Sin cara en la cama: evaluado en la orientación del modelado.",
        Text::PrintContact => "Apoyo",
        Text::PrintSmallBase => "Apoyo pequeño: la pieza puede despegarse de la cama.",
        Text::PrintRegions => "región(es)",
        Text::IssueOverhang => "Voladizos",
        Text::IssueThinWall => "Paredes delgadas",
        Text::IssueGap => "Rendijas",
        Text::BedFace => "Cara en la cama",
        Text::BedFaceShort => "Cara en la cama",
        Text::BedFaceField => "Cara apoyada",
        Text::BedFaceHint => {
            "Haga clic en una cara plana. La elección vale para la impresión y no cambia el modelado."
        }
        Text::BedSize => "En la cama",
        Text::BedTooSmall => "Mayor que la cama del perfil de impresión.",
        Text::BedRemove => "Volver a la orientación del modelado",
        Text::ExportForPrint => "Exportar para impresión",
        Text::ExportForPrintShort => "Exportar",
        Text::ExportFormat => "Formato",
        Text::ExportPrepareBed => "Preparar para la cama",
        Text::ExportOpenSlicer => "Abrir en el laminador",
        Text::ExportSlicerCommand => "Comando",
        Text::ExportSlicerHint => {
            "Vacío usa la aplicación predeterminada del sistema. {arquivo} marca dónde va el archivo."
        }
        Text::ExportNotOpened => "Exportado, pero el laminador no se abrió",
        Text::ExportOversized => "Mayor que la cama",
        Text::ExportOverflow => "El conjunto ocupa más de una cama",
        Text::ExportSkipped => "Superficies abiertas, fuera de la exportación",
        Text::ThreeMfFilter => "Modelo 3MF",
        Text::PrintProfile => "Perfil de impresión",
        Text::PrintProfileName => "Nombre",
        Text::PrintProfileLibrary => "Biblioteca",
        Text::PrintProfileChoose => "Cargar",
        Text::PrintProfileSave => "Guardar",
        Text::PrintProfileNoLibrary => "Carpeta de preferencias no disponible.",
        Text::PrintProfileHint => {
            "Los valores iniciales son reglas prácticas. Ajústelos con medidas de su impresora."
        }
        Text::PrintProfileNames => "En las expresiones",
        Text::PrintNozzle => "Boquilla",
        Text::PrintLayer => "Capa",
        Text::PrintFitTight => "Holgura ajustada",
        Text::PrintFitSliding => "Holgura deslizante",
        Text::PrintFitFree => "Holgura libre",
        Text::PrintHoleCompensation => "Compensar agujero",
        Text::PrintOverhang => "Voladizo máximo",
        Text::PrintDensity => "Densidad",
        Text::ComponentCut => "Recortar componente",
        Text::ComponentCutShort => "Recortar",
        Text::InsertionField => "Entra por",
        Text::FitClearance => "Holgura",
        Text::FloorClearance => "Holgura en el fondo",
        Text::ComponentDepth => "Longitud",
        Text::ComponentHint => {
            "Haga clic en el componente en la vista 3D. Puede estar en otra pieza."
        }
        Text::ComponentFrozen => {
            "Silueta capturada. Cambiar la holgura recalcula; mover el componente pide una nueva captura."
        }
        Text::Lip => "Resalte",
        Text::LipFace => "Cara del borde",
        Text::LipKindField => "Tipo",
        Text::LipRaised => "Añade",
        Text::LipGroove => "Rebaja",
        Text::LipReferenceField => "Contorno",
        Text::LipInner => "Interior",
        Text::LipOuter => "Exterior",
        Text::LipInset => "Separación",
        Text::LipHint => {
            "Use Añade en un cuerpo y Rebaja en el otro, con las mismas medidas: la holgura queda solo en la rebaja."
        }
        Text::FastenerHeight => "Altura",
        Text::ScrewBoss => "Torre de tornillo",
        Text::ScrewBossShort => "Torre",
        Text::BossOuter => "Diámetro exterior",
        Text::BossShallowHole => "Agujero menos profundo que la torre",
        Text::BaseFillet => "Acuerdo",
        Text::SnapFit => "Presilla",
        Text::SnapHook => "Presilla",
        Text::SnapWindow => "Ventana",
        Text::SnapLength => "Longitud",
        Text::SnapThickness => "Espesor",
        Text::SnapWidth => "Ancho",
        Text::SnapOverhang => "Avance",
        Text::SnapHead => "Rampa",
        Text::SnapWindowDepth => "Más allá de la cabeza",
        Text::SnapStrain => "Deformación estimada",
        Text::SnapStrainHigh => {
            "Por encima del límite del perfil: alargue el brazo o reduzca el avance."
        }
        Text::FitCheck => "Verificar encaje",
        Text::FitCheckShort => "Encaje",
        Text::FitChecking => "Comparando los cuerpos…",
        Text::FitInterference => "Interferencia",
        Text::FitTouching => "En contacto",
        Text::FitTight => "Holgura pequeña",
        Text::FitClean => "Sin interferencias.",
        Text::FitCompared => "Pares comparados",
        Text::FitBeyond => "Pares sin comparar, tiempo agotado",
        Text::FitSkipped => "Sin volumen que comparar",
        Text::FitFailed => "Error al comparar",
        Text::FitAgain => "Verificar de nuevo",
        Text::FitStale => "El ensamblaje cambió.",
        Text::FitScopeField => "Comparar",
        Text::FitScopePart => "Pieza en edición",
        Text::FitScopeAll => "Componentes",
        Text::ExcerptToggle => "Solo un tramo",
        Text::ExcerptHint => "Caja en coordenadas de la pieza. El modelo no cambia.",
        Text::PrintMass => "Masa estimada",
        Text::PrintCost => "Coste estimado",
        Text::PrintPrice => "Precio por kg",
        Text::PrintInfill => "Relleno",
        Text::PrintWallLines => "Vueltas de pared",
        Text::PrintMaxStrain => "Deformación máxima",
        Text::PrintEstimateHint => "Estimación antes de laminar, sin soportes.",
        Text::PrintBedLimit => "Limitar al tamaño de la cama",
        Text::SettingsMenu => "Configuración",
        Text::SettingsAppearance => "Apariencia",
        Text::SettingsDrawing => "Dibujo",
        Text::SettingsShortcuts => "Atajos",
        Text::SettingsAbout => "Acerca de",
        Text::SettingsAboutDescription => {
            "CAD paramétrico en Rust para modelar piezas mecánicas precisas para prototipos."
        }
        Text::SettingsAboutVersion => "Versión",
        Text::SettingsAboutAuthor => "Autor",
        Text::SettingsAboutAuthorValue => "Bruno Trócolis",
        Text::SettingsAboutLicense => "Licencia",
        Text::SettingsAboutLicenseValue => "MIT",
        Text::SettingsAboutRepository => "Repositorio",
        Text::SettingsAboutRepositoryLink => "github.com/trocolis/3dnator",
        Text::SettingsAboutProjectFormat => "Formato del proyecto",
        Text::SettingsAboutTechnologies => "Tecnologías",
        Text::SettingsAboutTechnologiesValue => "Rust · egui/wgpu · Open CASCADE Technology",
        Text::SettingsAboutCopyright => "© 2026 Bruno Trócolis",
        Text::SettingsUnit => "Unidad de medida",
        Text::SettingsUnitMillimeter => "Milímetro (mm)",
        Text::SettingsUnitInch => "Pulgada (in)",
        Text::SettingsUnitHint => {
            "Las medidas y los números escritos sin unidad usan esta escala; la geometría sigue en milímetros."
        }
        Text::SettingsMouseStep => "Precisión del ratón",
        Text::SettingsMouseAngleStep => "Precisión de ángulos",
        Text::SettingsMouseStepHint => {
            "Al dibujar, mover, girar o extruir, el ratón avanza en múltiplos de la precisión. Escriba una medida para usar cualquier valor; en el boceto, Alt libera el cursor."
        }
        Text::ThroughShell => "Atravesar (crear conducto)",
        Text::CuttingPlane => "Plano de división",
        Text::RevolveAxisHint => {
            "Haga clic en una línea de un boceto coplanar con las áreas, incluida una línea de referencia o infinita."
        }
        Text::CoplanarProfilesRequired => "Seleccione áreas y eje en el mismo plano.",
        Text::Loft => "Transición entre perfiles",
        Text::Pipe => "Tubo",
        Text::PipeDiameter => "Diámetro exterior",
        Text::PipeHollow => "Tubo hueco",
        Text::PipeHint => {
            "Tubo por trayectoria: haz clic en un círculo, línea o arco del boceto. Admite caminos abiertos o cerrados con curvas tangentes. Desactiva Tubo hueco para una sección maciza."
        }
        Text::Sweep => "Transición guiada",
        Text::SweepHint => {
            "Guía automática, arco alrededor de un eje en el plano inicial o cadena abierta de líneas/arcos de un boceto. Los planos de los perfiles deben ser perpendiculares a la guía en los extremos."
        }
        Text::RouteField => "Trayectoria",
        Text::AutomaticRoute => "Automática",
        Text::LongArc => "Arco por el lado opuesto",
        Text::EndProfile => "Área final",
        Text::StartProfile => "Área inicial",
        Text::RuledLoft => "Generatrices rectas",
        Text::DevelopableLoft => "Superficie desarrollable",
        Text::DevelopableLoftHint => {
            "De polígono convexo a círculo: triángulos planos y conos, sin torsión, que admiten vaciado. Otros perfiles usan la transición común."
        }
        Text::UpdateHolePairs => "Actualizar correspondencia de agujeros",
        Text::LoftHint => {
            "Elija un área en cada plano. Los agujeros deben corresponder por orden de creación y existir en ambos perfiles."
        }
        Text::LoftDifferentPlanes => "Elija el área final en un plano distinto del inicial.",
        Text::CuttingPlaneHint => {
            "Elija una cara plana, un plano de trabajo o un plano del origen. Se conservarán ambos lados."
        }
        Text::ExitFace => "Cara de salida",
        Text::AutomaticExitHint => {
            "Sin selección: usar la primera cara opuesta única. Elija la salida en el mismo cuerpo."
        }
        Text::SketchXZ => "Boceto XZ",
        Text::SketchYZ => "Boceto YZ",
        Text::SearchTools => "Buscar herramientas",
        Text::SearchHint => "Escriba una herramienta…",
        Text::NoMatchingTools => "No se encontró ninguna herramienta.",
        Text::ChooseActivePart => "Seleccione una pieza y use Construir en esta pieza.",
        Text::DocumentGroup => "Documentar",
        Text::ActivityGeometry => "Crear geometría",
        Text::ActivityRelation => "Editar relaciones",
        Text::ActivityModify => "Modificar geometría",
        Text::BrowserEmpty => "Su proyecto empieza aquí",
        Text::StartSketch => "Cree un boceto o importe un modelo.",
        Text::SolidBodies => "Cuerpos",
        Text::SketchesGroup => "Bocetos",
        Text::ParentPart => "Insertar dentro de la pieza",
        Text::RootAssembly => "Raíz del ensamblaje",
        Text::Groups => "Grupos organizativos",
        Text::GroupMembers => "Seleccione instancias del mismo nivel",
        Text::SolidBody => "Sólido CAD",
        Text::ParametricHistory => "Historial",
        Text::ViewOrbitReverseHint => {
            "Paso de 45° alrededor de la normal de la cara predominante. Mayús+clic o botón derecho invierte el sentido."
        }
        Text::ZeroExtrusionHint => {
            "Cero divide la cara por el perfil, conservando el volumen del cuerpo."
        }
        Text::Extrude => "Extrusión",
        Text::Revolve => "Revolución",
        Text::Join => "Unir",
        Text::Cut => "Cortar",
        Text::Intersect => "Intersección",
        Text::Fillet => "Redondeo",
        Text::Chamfer => "Chaflán",
        Text::Shell => "Vaciado",
        Text::Profile => "Boceto de perfil",
        Text::NoRegions => "El boceto no tiene regiones cerradas utilizables.",
        Text::Angle => "Ángulo / cota",
        Text::TopologyIndex => {
            "Índice de arista/cara en el cuerpo base; al aplicar, se convierte en referencia por las caras de origen"
        }
        Text::KeptReference => "Referencia actual (se mantiene mientras no se cambie el índice)",
        Text::BodyColor => "Color del cuerpo",
        Text::NeutralColor => "Restaurar color neutro",
        Text::Isolate => "Aislar",
        Text::ExitIsolation => "Salir del aislamiento",
        Text::TransparentToggle => "Transparente",
        Text::NewBody => "Nuevo cuerpo",
        Text::ExportScope => "Qué exportar",
        Text::ScopeBody => "Cuerpo seleccionado",
        Text::ScopePart => "Cuerpos de la pieza",
        Text::ScopeDefinitions => "Cada definición una vez",
        Text::ScopeJoined => "Ensamblaje entero como un objeto",
        Text::ScopeOccurrences => "Todas las instancias",
        Text::ExportQuality => "Calidad",
        Text::QualityDraft => "Borrador",
        Text::QualityStandard => "Estándar",
        Text::QualityHigh => "Alta",
        Text::QualityCustom => "Personalizada",
        Text::LinearTolerance => "Desviación lineal (mm)",
        Text::AngularTolerance => "Desviación angular (°)",
        Text::ComputeSummary => "Calcular resumen",
        Text::ComputeSummaryHint => "Calcule el resumen para ver archivos, tamaño y dimensiones.",
        Text::ExportFiles => "Exportar…",
        Text::ExportStep => "Exportar STEP…",
        Text::ExportPartStep => "Pieza en STEP…",
        Text::ExportAssemblyStep => "Ensamblaje en STEP…",
        Text::FilesLabel => "objeto(s)",
        Text::Occurrences => "instancias en el ensamblaje",
        Text::FaceLabel => "Cara",
        Text::EdgeLabel => "Arista",
        Text::Length => "Longitud",
        Text::DiameterLabel => "Diámetro",
        Text::AngleLabel => "Ángulo hasta la referencia",
        Text::DrawingRevision => "Revisión",
        Text::DrawingDate => "Fecha",
        Text::SketchOnFace => "En la cara seleccionada (plano capturado)",
        Text::MirroredSuffix => "simétrica",
        Text::HoleTool => "Agujero",
        Text::LinearPattern => "Patrón lineal",
        Text::CircularPattern => "Patrón circular",
        Text::MirrorBody => "Simetría de cuerpo",
        Text::SplitBody => "Dividir cuerpo",
        Text::SplitShort => "Dividir",
        Text::CheckSolid => "Verificar sólido",
        Text::CheckSolidShort => "Verificar",
        Text::CheckSolidHint => "Elija el cuerpo a verificar.",
        Text::CheckingSolid => "Analizando el cuerpo…",
        Text::SolidHasNoDefects => "No se encontraron defectos: el cuerpo se puede exportar.",
        Text::SolidMeshDefects => {
            "El STL de este cuerpo tendría defectos, pero no hay contactos reparables. Busque rendijas microscópicas entre bocetos casi alineados."
        }
        Text::SolidSliver => "Astilla menor que la precisión del STL",
        Text::SolidSliverWidth => "Ancho",
        Text::SolidSliverHint => {
            "No cambia la pieza: la astilla sale de la malla al exportar. Para eliminarla del modelo, ajuste el boceto de la operación indicada."
        }
        Text::PrintSlivers => {
            "triángulos menores que la precisión del STL salen de la malla al exportar, cerca de"
        }
        Text::Contact => "Contacto",
        Text::ContactOrigin => "Surgió en",
        Text::RepairContact => "Reparación de contacto",
        Text::RepairModeField => "Reparación",
        Text::WeldContact => "Soldar",
        Text::WeldContactHint => "Une los dos volúmenes añadiendo material a lo largo del contacto",
        Text::SeparateContact => "Separar",
        Text::SeparateContactHint => {
            "Separa los dos volúmenes quitando material a lo largo del contacto"
        }
        Text::HolePoints => "Centros de círculo y puntos aislados del boceto",
        Text::NoHolePoints => "El boceto no tiene centros de círculo ni puntos aislados.",
        Text::HoleDiameter => "Diámetro",
        Text::HoleDepth => "Profundidad",
        Text::Through => "Pasante",
        Text::HeadSimple => "Simple",
        Text::Counterbore => "Avellanado recto",
        Text::Countersink => "Avellanado cónico",
        Text::HeadDiameter => "Diámetro de la cabeza",
        Text::HeadDepth => "Profundidad del avellanado recto",
        Text::HeadAngle => "Ángulo del avellanado",
        Text::DrillPoint => "Punta de broca",
        Text::ReverseDirection => "Invertir dirección",
        Text::ThreadToggle => "Rosca simplificada (registrada, sin hélice)",
        Text::ThreadDesignation => "Designación",
        Text::ThreadNominal => "Diámetro nominal",
        Text::ThreadPitch => "Paso",
        Text::SecondDirection => "Segunda dirección",
        Text::TotalAngle => "Ángulo total",
        Text::KeepOriginal => "Conservar el original (pieza simétrica)",
        Text::FlipSide => "Conservar el otro lado",
        Text::PickOtherPart => "El cuerpo seleccionado pertenece a otra pieza.",
        Text::KernelUnavailable => {
            "Se requiere el SDK de OCCT: compile con la feature occt para construir sólidos."
        }
        Text::ModelFilter => "Modelos STL, STEP o 3MF",
        Text::StepFilter => "Geometría STEP",
        Text::OutputExists => "El destino ya existe; elija un nombre nuevo.",
        #[cfg(feature = "occt")]
        Text::MeshIsNotSolid => "La malla no tiene geometría CAD precisa para exportar STEP.",
        Text::Tangent => "Tangencia",
        Text::Drawing => "Plano técnico",
        Text::DrawingTitle => "Título",
        Text::Paper => "Papel",
        Text::Blueprint => "Estilo blueprint",
        Text::HiddenLines => "Líneas ocultas",
        Text::DrawingDimensions => "Cotas generales de las vistas",
        Text::Preview => "Actualizar vista previa",
        Text::SaveSheet => "Guardar hoja en el proyecto",
        Text::ExportPdf => "Exportar PDF vectorial",
        Text::ExportPng => "Exportar PNG",
        Text::Section => "Vista en corte A–A",
        Text::SectionPosition => "Origen del plano de corte (mm)",
        Text::PlaneNormal => "Normal del plano de corte",
        Text::AssociativeDimensions => "Cotas asociativas",
        Text::DimensionKind => "Tipo de cota",
        Text::DimensionView => "Vista de la cota",
        Text::DimensionLength => "Longitud de la arista",
        Text::DimensionRadiusFace => "Radio de la cara",
        Text::DimensionDiameterFace => "Diámetro de la cara",
        Text::DimensionRadiusEdge => "Radio de la arista circular",
        Text::DimensionDiameterEdge => "Diámetro de la arista circular",
        Text::DimensionFaceDistance => "Distancia entre caras paralelas",
        Text::DimensionFaceAngle => "Ángulo entre caras planas",
        Text::CaptureFirstReference => "Capturar primera referencia seleccionada",
        Text::CaptureSecondReference => "Capturar segunda referencia seleccionada",
        Text::AddDimension => "Añadir cota",
        Text::RemoveDimension => "Quitar cota",
        Text::UpdateDimensionReferences => "Aplicar referencias a la cota elegida",
        Text::DimensionTextPosition => {
            "Desplazamiento del texto en el papel (mm); también puede arrastrarlo en la vista previa"
        }
        Text::SupplementaryAngle => "Ángulo suplementario",
        Text::DimensionReview => "Cotas que necesitan revisión",
        Text::StaleDrawingPreview => {
            "Vista previa anterior al cambio. Actualícela antes de reposicionar las cotas."
        }
        Text::DrawingReferenceMissing => "Referencia eliminada",
        Text::AddSection => "Añadir corte",
        Text::RemoveSection => "Quitar corte",
        Text::Portrait => "Vertical",
        Text::PngResolution => "Resolución PNG (DPI)",
        Text::MeshDrawingNotice => {
            "La leyenda identifica vistas del B-rep o aproximadas de la malla. Los cortes de malla son aproximados; las cotas generales miden los límites proyectados."
        }
        Text::Title => "3Dnator · Proyectos",
        Text::NewProject => "Nuevo",
        Text::OpenProject => "Abrir proyecto",
        Text::Save => "Guardar",
        Text::SaveAs => "Guardar como…",
        Text::RecentProjects => "Proyectos recientes",
        Text::Undo => "Deshacer",
        Text::Redo => "Rehacer",
        Text::Import => "Importar modelo",
        Text::Reference => "Insertar soporte en L",
        Text::Project => "Árbol del proyecto",
        Text::Untitled => "Sin título",
        Text::MeshBody => "Malla",
        Text::Triangles => "Triángulos",
        Text::Triangulation => "Mostrar triangulación",
        Text::Shadows => "Sombras",
        Text::Projection => "Proyección",
        Text::Orthographic => "Ortográfica",
        Text::Perspective => "Perspectiva",
        Text::Fit => "Encuadrar",
        Text::Front => "Frontal",
        Text::Top => "Superior",
        Text::Side => "Lateral",
        Text::Iso => "Isométrica",
        Text::Loading => "Procesando…",
        Text::Ready => "Listo",
        Text::Navigation => "Rueda: zoom · Central: desplazar · Mayús + central: orbitar",
        Text::Scale => "Escala STL a milímetros",
        Text::Failed => "La operación falló",
        Text::Export => "Exportar STL…",
        Text::Exported => "Exportación terminada",
        Text::Saved => "Proyecto guardado",
        Text::Imported => "Cuerpo importado",
        Text::Assembly => "Ensamblaje",
        Text::Construction => "Referencia",
        Text::EditPart => "Construir en esta pieza",
        Text::Position => "Posición X / Y / Z (mm)",
        Text::Close => "Cerrar",
        Text::Unsaved => "Cambios sin guardar",
        Text::SaveChanges => "¿Guardar los cambios antes de cerrar?",
        Text::Discard => "Descartar",
        Text::Cancel => "Cancelar",
        Text::Recovery => "Recuperación",
        Text::Recover => "Recuperar",
        Text::RecoverySaved => "Recuperación automática actualizada",
        Text::RecoveryFailed => "No se pudo actualizar la recuperación",
        Text::EmptyScene => "Cree un boceto o importe un modelo para empezar.",
        Text::FileFilter => "Proyecto 3Dnator",
        Text::StlFilter => "Cuerpo STL",
        Text::RecoveryNotice => {
            "Hay copias de recuperación. Abra una para revisar y guardar el proyecto."
        }
        Text::FileAlreadyOpen => "Este archivo ya está abierto en otra pestaña.",
        Text::StaleResult => "Resultado ignorado: el proyecto cambió durante la operación.",
        Text::Sketch => "Boceto",
        Text::AutomaticExtrusion => "Operación automática",
        Text::ExtrusionProfileHint => {
            "Haga clic en un área: región del boceto, parte de la cara delimitada por él o cara plana. Haga clic de nuevo para quitarla. Otro plano o cara pasa a ser el límite."
        }
        Text::ExtrusionMouseHint => {
            "La extrusión empieza en 0 mm. Arrastre la flecha o escriba la distancia. Haga clic en otro plano para terminar en él. Elegir una operación desactiva la sugerencia automática."
        }
        Text::RemoveMaterial => "Quitar",
        Text::FindingClosedAreas => "Identificando áreas cerradas…",
        Text::ProfileQueryFailed => "No se pudieron identificar las áreas cerradas.",
        Text::SketchSweep => "Apertura",
        Text::SketchDynamicWidth => "Ancho",
        Text::SketchDynamicHeight => "Alto",
        Text::SketchValueLocked => {
            "Medida bloqueada por la escritura. Borre el campo para volver a seguir el cursor."
        }
        Text::SketchDynamicHint => "Tab: alternar · Enter: confirmar · Esc: cancelar",
        Text::SketchLineMode => "Tipo de línea",
        Text::SketchRectangleMode => "Tipo de rectángulo",
        Text::SketchCircleMode => "Tipo de círculo",
        Text::SketchRectangleCenter => "Desde el centro hacia fuera",
        Text::SketchRectangleInclined => "Inclinado",
        Text::SketchCircleDiameter => "Dos puntos (diámetro)",
        Text::SketchCircleTangent => "Tangente",
        Text::SketchContinuousLineHint => {
            "Haga clic en los siguientes vértices. Esc termina la secuencia; cerrar un área o terminar sobre otro elemento también la termina. Empezando en un círculo o arco, mantenga Shift para deslizar el origen sobre él con la línea tangente."
        }
        Text::SketchSymmetricLineHint => {
            "El primer punto es el centro: cada segmento crea otro del mismo tamaño en el lado opuesto."
        }
        Text::SketchRectangleNormalHint => "Haga clic en dos esquinas opuestas.",
        Text::SketchRectangleCenterHint => {
            "Haga clic en el centro y después en una esquina. Las medidas indican el ancho y el alto totales."
        }
        Text::SketchRectangleInclinedHint => {
            "Haga clic en el inicio y el fin de un lado; después defina el ancho perpendicular."
        }
        Text::SketchCircleNormalHint => {
            "Haga clic en el centro y después en el borde para definir el radio."
        }
        Text::SketchCircleDiameterHint => "Haga clic en los dos extremos del diámetro.",
        Text::SketchCircleTangentHint => {
            "Seleccione dos líneas perpendiculares. Mueva el ratón para elegir el cuadrante y el radio, o escriba el radio; clic o Enter confirma. Usa las rectas soporte."
        }
        Text::SketchPickFirstTangent => "Seleccione la primera línea de apoyo.",
        Text::SketchPickSecondTangent => "Seleccione la segunda línea, perpendicular a la primera.",
        Text::SketchPickLine => "Seleccione una línea del boceto.",
        Text::SketchLineType => "Trazo",
        Text::SketchLineNormal => "Normal",
        Text::SketchLineInfinite => "Infinita",
        Text::SketchLineNormalHint => "Geometría de dibujo; participa en áreas y extrusiones.",
        Text::SketchLineConstructionHint => {
            "Geometría de referencia discontinua naranja; no define áreas ni extrusiones."
        }
        Text::SketchLineInfiniteHint => {
            "Recta discontinua naranja sin extremos; solo sirve de referencia."
        }
        Text::SketchRelativeAngleHint => {
            "Ángulo respecto a la línea capturada en el primer punto; positivo en sentido antihorario."
        }
        Text::SketchAngleValue => "Ángulo (°)",
        Text::SketchHorizontalAngleHint => {
            "Ángulo respecto al eje horizontal del boceto; positivo en sentido antihorario."
        }
        Text::SketchDimension => "Acotar",
        Text::HorizontalVertical => "Horizontal/vertical",
        Text::SketchExtendShort => "Extender",
        Text::SketchValue => "Valor (mm)",
        Text::SketchEntities => "Entidades",
        Text::SketchSelectOne => "Seleccione una entidad en el boceto.",
        Text::SketchSelectTwo => "Seleccione dos entidades en el boceto, en el orden indicado.",
        Text::SketchSelectDimension => "Seleccione una cota o una línea, círculo o arco.",
        Text::SketchMirrorSelectionHint => {
            "Seleccione los elementos en el campo Entidades y elija una línea en el campo Eje de simetría."
        }
        Text::SketchTangentSelectionHint => {
            "Seleccione una línea y un arco con un extremo compartido."
        }
        Text::SketchCoincidentSelectionHint => {
            "Seleccione dos entidades. Sus extremos más cercanos (o centros de círculos) serán coincidentes."
        }
        Text::Parameters => "Parámetros",
        Text::ParameterName => "Nombre",
        Text::Expression => "Expresión",
        Text::CurrentValue => "Valor actual",
        Text::LengthKind => "Longitud",
        Text::AngleKind => "Ángulo",
        Text::ScalarKind => "Sin unidad",
        Text::ExpressionHint => {
            "Ejemplos: ancho / 2; 10 mm; 1 in. Use unidades explícitas en las sumas."
        }
        Text::ApplyParameter => "Aplicar parámetro",
        Text::DeleteParameter => "Eliminar parámetro",
        Text::Fixed => "Punto fijo",
        Text::Coincident => "Coincidencia",
        Text::Horizontal => "Horizontal",
        Text::Vertical => "Vertical",
        Text::HorizontalDimension => "Distancia horizontal",
        Text::VerticalDimension => "Distancia vertical",
        Text::Radius => "Radio",
        Text::Midpoint => "Punto medio",
        Text::PointOnLine => "Punto sobre línea",
        Text::Concentric => "Concentricidad",
        Text::EditDimension => "Cambiar cota seleccionada",
        Text::NewSketch => "Nuevo boceto",
        Text::SketchConflict => "Restricciones en conflicto — cambio pendiente",
        Text::FullyConstrained => "Totalmente restringido",
        Text::UnderConstrained => "Libre o parcialmente restringido",
        Text::DegreesOfFreedom => "Grados de libertad",
        Text::Redundant => "Hay restricciones redundantes",
        Text::Select => "Seleccionar",
        Text::Line => "Línea",
        Text::Rectangle => "Rectángulo",
        Text::Circle => "Círculo",
        Text::Arc => "Arco: centro, inicio y fin",
        Text::Trim => "Recortar",
        Text::Dimension => "Valor de la cota / desplazamiento",
        Text::ReferenceDimension => "Cota de referencia",
        Text::Entity => "Entidad",
        Text::FixPoint => "Fijar primer punto / centro",
        Text::ConstructionGeometry => "Alternar trazo Normal/Referencia",
        Text::DeleteEntity => "Eliminar entidades seleccionadas",
        Text::Parallel => "Paralelas",
        Text::Perpendicular => "Perpendiculares",
        Text::EqualLength => "Longitudes iguales",
        Text::Constraints => "Cotas y restricciones",
        Text::FinishSketch => "Terminar boceto",
        Text::FinishSketchShort => "Terminar",
        Text::CancelSketch => "Cancelar boceto",
        Text::InvalidArc => "Arco no válido: elija centro y extremos distintos.",
        Text::PendingEdit => "Termine o cancele el cambio pendiente antes de guardar.",
        Text::TimelineFirst => "Ir al inicio del historial",
        Text::TimelinePrevious => "Retroceder una operación",
        Text::TimelineNext => "Avanzar una operación",
        Text::TimelineLast => "Ir al final del historial",
        Text::HistoryMarker => "Marcador del historial: arrastre para volver a una etapa",
        Text::EditFeature => "Editar operación",
        Text::EditProfileSketch => "Editar boceto del perfil",
        Text::RenameFeature => "Renombrar",
        Text::SuppressFeature => "Suprimir",
        Text::UnsuppressFeature => "Reactivar",
        Text::MarkerHere => "Mover marcador aquí",
        Text::DeleteFeature => "Eliminar",
        Text::StateActive => "Activa",
        Text::StateSuppressed => "Suprimida",
        Text::StateDependent => "Inactiva: depende de una operación suprimida",
        Text::StateRolledBack => "Después del marcador del historial",
        Text::StateFailed => "Falló al recalcular la edición pendiente",
        Text::StateAffected => "Depende de la operación que falló",
        Text::DeleteFeatureTitle => "Eliminar operación",
        Text::DeleteDependents => {
            "Estas operaciones dependen del resultado y también se eliminarán:"
        }
        Text::DeleteAll => "Eliminar todas",
        Text::RebuildFailedTitle => "Error en el recálculo",
        Text::RebuildFailedAffected => "Operaciones afectadas:",
        Text::FixFeature => "Editar la operación",
        Text::SuppressAndApply => "Suprimir y aplicar",
        Text::CancelEdit => "Cancelar edición",
        Text::PendingEditBanner => {
            "Edición pendiente: se muestra el último resultado válido, anterior a ella."
        }
        Text::FixingFailure => {
            "Corrigiendo la operación que falló; al aplicar, la edición pendiente entra junto."
        }
        Text::RenameFeatureTitle => "Renombrar operación",
        Text::TimelineDragHint => "Arrastre para reordenar; haga clic derecho para más opciones.",
        Text::Ok => "Aceptar",
        Text::PreviewComputing => "Calculando vista previa…",
        Text::PreviewReady => "Vista previa actualizada",
        Text::PreviewFailed => "No se pudo calcular",
        Text::SelectPrompt => "Seleccionar",
        Text::SelectedCount => "seleccionados",
        Text::ClearSelection => "Limpiar selección",
        Text::ProfileField => "Perfil",
        Text::ProfileHint => "Haga clic en una región cerrada del boceto en la vista 3D.",
        Text::DirectionField => "Dirección",
        Text::OneSide => "Un lado",
        Text::TwoSides => "Dos lados",
        Text::Symmetric => "Simétrica",
        Text::ExtentField => "Extensión",
        Text::ExtentDistance => "Distancia",
        Text::ExtentToFace => "Hasta un plano",
        Text::DistanceField => "Distancia",
        Text::Distance2Field => "Distancia 2",
        Text::StartOffset => "Desplazamiento inicial",
        Text::FlipDirection => "Invertir dirección",
        Text::OperationField => "Operación",
        Text::TargetBody => "Cuerpo objetivo",
        Text::EdgesField => "Aristas",
        Text::FacesField => "Caras",
        Text::RadiusField => "Radio",
        Text::ThicknessField => "Espesor",
        Text::ChamferType => "Tipo",
        Text::EqualDistance => "Distancia igual",
        Text::DistanceAngle => "Distancia y ángulo",
        Text::InwardWalls => "Hacia dentro",
        Text::OutwardWalls => "Hacia fuera",
        Text::AxisField => "Eje",
        Text::FullTurn => "Vuelta completa",
        Text::BodyField => "Cuerpo",
        Text::ToolBodyField => "Cuerpo herramienta",
        Text::CountField => "Cantidad",
        Text::SpacingField => "Espaciado",
        Text::PlaneField => "Plano",
        Text::OffsetField => "Desplazamiento",
        Text::OriginField => "Origen",
        Text::AdvancedGroup => "Avanzado",
        Text::FromField => "Inicio",
        Text::ToField => "Fin",
        Text::HoleType => "Tipo de agujero",
        Text::HoleExtent => "Extensión",
        Text::CombineTool => "Combinar",
        Text::RectangularPattern => "Patrón rectangular",
        Text::SolidBodyRequired => {
            "Seleccione un sólido CAD; los cuerpos de malla no participan en operaciones."
        }
        Text::SelectOtherTitle => "Seleccionar entre superpuestos",
        Text::CopySuffix => "copia",
        Text::GroupLabel => "Grupo",
        Text::MoveCopyTool => "Mover/Copiar",
        Text::AlignTool => "Alinear",
        Text::CircularInstancePattern => "Patrón circular de componentes",
        Text::RectangularInstancePattern => "Patrón rectangular de componentes",
        Text::MirrorComponent => "Simetría de componente",
        Text::ObjectsField => "Objetos",
        Text::TranslationField => "Traslación",
        Text::RotationField => "Rotación",
        Text::PivotField => "Pivote",
        Text::PivotCenter => "Centro de masa",
        Text::PivotOrigin => "Origen del primer elemento",
        Text::CreateCopy => "Crear copia (nueva instancia o cuerpo)",
        Text::ResetTransform => "Restablecer",
        Text::MoveHint => {
            "Arrastre las flechas para trasladar, los anillos para girar y el centro para mover libremente."
        }
        Text::InvalidTransformNumber => {
            "Corrija los campos resaltados. Use números o expresiones entre −10.000.000 y 10.000.000 en la unidad indicada; Esc restaura el último valor válido."
        }
        Text::AlignFrom => "Origen",
        Text::AlignTo => "Destino",
        Text::AlignHint => {
            "Haga clic en la geometría del elemento que se mueve y después en la de destino, en otro componente o en un boceto de referencia."
        }
        Text::ComponentField => "Componente",
        Text::MirrorHint => {
            "Crea una pieza nueva e independiente, reflejada por el plano en coordenadas del padre."
        }
        Text::CloneTool => "Clon independiente",
        Text::GroupTool => "Agrupar",
        Text::UngroupTool => "Desagrupar",
        Text::Selectable => "Seleccionable",
        Text::Hide => "Ocultar",
        Text::Show => "Mostrar",
        Text::RenameDefinition => "Renombrar definición de la pieza",
        Text::SelectMembers => "Seleccionar miembros",
        Text::StepShort => "STEP",
        Text::SectionAnalysis => "Corte de sección",
        Text::PickedFace => "Cara elegida",
        Text::SectionFaceHint => "Usa el plano de la cara elegida en la vista 3D, capturado ahora.",
        Text::SectionHint => {
            "Arrastre la flecha para desplazar el plano. El corte es solo visual: no cambia el modelo ni el deshacer, y la selección ignora la parte cortada."
        }
        Text::AnalysesGroup => "Análisis",
        Text::EditFeatureShort => "Editar",
        Text::RepairSketch => "Reparar boceto",
        Text::RepairTolerance => "Holgura máxima",
        Text::RepairDuplicate => "Eliminar entidad duplicada",
        Text::RepairContained => "Eliminar tramo superpuesto",
        Text::RepairGap => "Unir extremos abiertos",
        Text::RepairLinkEnds => "Unir extremos en el mismo lugar",
        Text::RepairHorizontal => "Hacer horizontal",
        Text::RepairVertical => "Hacer vertical",
        Text::RepairCrossing => "Cruce en",
        Text::RepairCrossingHint => "use Recortar para elegir el tramo",
        Text::RepairNothing => "No se encontró ningún problema corregible.",
        Text::RepairHint => {
            "Las correcciones marcadas aparecen en naranja en el boceto y solo se aplican con Aceptar. Unir extremos añade una coincidencia resuelta por el solver."
        }
        Text::DisplayDecimals => "Decimales mostrados",
        Text::DisplayDecimalsHint => {
            "Afecta a medidas, propiedades y parámetros en la interfaz; la precisión interna no cambia."
        }
        Text::InsertFromProject => "Pieza de otro proyecto",
        Text::NoOtherProjects => "Abra otro proyecto en una pestaña para insertar sus piezas.",
        Text::SourceProject => "Proyecto de origen",
        Text::PartsField => "Piezas disponibles",
        Text::SubpartsCount => "subpiezas",
        Text::InsertIntoPart => "La copia entra como subpieza de",
        Text::InsertAtRoot => {
            "La copia entra en la raíz del ensamblaje, independiente del archivo de origen."
        }
        Text::SketchFaceShort => "En la cara",
        Text::PivotPicked => "Punto elegido en la vista 3D (vértice, arista o cara)",
        Text::InsertMesh => "Insertar malla STL",
        Text::FileUnit => "Unidad del archivo",
        Text::CustomScale => "Factor",
        Text::FileSize => "Dimensiones leídas",
        Text::ResultSize => "Dimensiones en mm",
        Text::MeshImportHint => {
            "El STL no indica la unidad. Compruebe las dimensiones antes de insertar; la malla entra como cuerpo de referencia, sin conversión a sólido."
        }
        Text::ReferenceSketch => "Boceto de referencia",
        Text::NewPartMenu => "Nueva pieza",
        Text::AddFaceLines => "Traer líneas de la cara",
        Text::ShortcutsSystem => "Sistema",
        Text::ShortcutsWorkspace => "Construcción y ensamblaje",
        Text::ShortcutsSketch => "Boceto",
        Text::ShortcutsModel => "Construcción",
        Text::ShortcutsAssembly => "Ensamblaje",
        Text::ShortcutStroke => "Alternar trazo de referencia",
        Text::ShortcutConflict => "Esa tecla ya se usa en este contexto",
        Text::ShortcutsHint => {
            "Haga clic en la tecla de una acción y pulse la nueva combinación; Esc desiste. La misma letra puede valer en contextos distintos."
        }
        Text::ShortcutWaiting => "Pulse la tecla…",
        Text::ShortcutsReset => "Restaurar valores predeterminados",
        Text::SettingsTheme => "Tema",
        Text::SettingsSystem => "Sistema",
        Text::SettingsThemeLight => "Claro",
        Text::SettingsThemeDark => "Oscuro",
        Text::SettingsLanguage => "Idioma",
        Text::SettingsConfigure => "Configurar",
        Text::SettingsShortcutsHint => {
            "Teclas que activan las herramientas, separadas por contexto."
        }
        Text::SettingsThemeHint => {
            "Vale para todos los proyectos en este equipo. Sistema sigue el tema del sistema operativo."
        }
        Text::SettingsLanguageHint => {
            "Vale para todos los proyectos en este equipo. Sistema sigue el idioma del sistema operativo; fuera de portugués, español e inglés, se usa el portugués."
        }
        Text::SettingsAboutButton => "Acerca de 3dnator",
        Text::SketchTextX => "Posición X",
        Text::SketchTextY => "Posición Y",
        Text::SketchTextAdjustHint => "Ajusta las propiedades y confirma para aplicar el texto.",
        Text::SketchTextTool => "Texto",
        Text::SketchTextHint => "Haga clic en el punto de inserción del texto, en su centro.",
        Text::SketchTextDirectionHint => {
            "Haga clic en la dirección del texto, o confirme con la rotación del panel."
        }
        Text::ReferencesFolder => "Referencias",
        Text::ReferenceModelHint => {
            "Modelo importado como referencia: se puede mover, ocultar y medir, pero no editar."
        }
        Text::ConvertToReference => "Convertir en objeto de referencia",
        Text::ConvertToReferenceHint => {
            "Se aplica a todas las instancias de esta pieza. Confirma o cancela la herramienta abierta antes de convertir."
        }
        Text::CreatePartFromReference => "Crear pieza",
        Text::ReferenceNotEditable => {
            "Las referencias no se pueden editar. Use Crear pieza en su menú, en el árbol."
        }
        Text::MergeCoplanarAreas => "Unir áreas coplanares",
        Text::ImportMenu => "Importar",
        Text::ImportReference => "Importar STEP, STL o 3MF como referencia",
        Text::MovementMode => "Modo de movimiento",
        Text::MoveMenu => "Mover",
        Text::MoveFree => "Libre",
        Text::MoveFaceToFace => "Cara a cara",
        Text::MoveEdgeToEdge => "Arista a arista",
        Text::MovePointToPoint => "Vértice a vértice",
        Text::CopyMenu => "Copiar",
        Text::CopyType => "Tipo de copia",
        Text::CopyNewInstance => "Nueva instancia",
        Text::CopyIndependent => "Copia independiente",
        Text::NewPartName => "Pieza",
        Text::MoveFreeHint => "Manipulador 3D y valores de traslación y rotación.",
        Text::MoveFaceHint => {
            "Caras opuestas y coplanares; acepta el plano de un boceto de referencia como destino."
        }
        Text::MoveEdgeHint => {
            "Aristas paralelas; acepta líneas de bocetos de referencia como destino."
        }
        Text::MovePointHint => {
            "Vértices, puntos medios, centros de círculos y arcos y cuadrantes del borde: acerque el cursor para capturar. Acepta puntos de bocetos de referencia como destino."
        }
        Text::VirtualCenter => "Centro circular (virtual)",
        Text::VirtualArcCenter => "Centro del arco (virtual)",
        Text::EdgeMidpoint => "Punto medio de la arista",
        Text::FaceCenter => "Centro de la cara",
        Text::CircleBoundaryPoint => "Cuadrante del borde circular",
        Text::PointQueryFailed => "No se pudieron consultar los puntos de captura.",
        Text::CopyInstanceHint => {
            "Misma definición, al lado de la original: editar una cambia todas."
        }
        Text::CopyIndependentHint => {
            "Pieza nueva al lado de la original, con historial propio y sin vínculo."
        }
        Text::CopyPatternHint => {
            "Instancias repetidas en patrón rectangular o circular, reunidas en un grupo."
        }
        Text::ReferenceSketchHint => {
            "Boceto del ensamblaje para posicionar piezas; no genera cuerpos."
        }
        Text::LeavePart => "Volver al ensamblaje",
        Text::ProjectNode => "Proyecto",
        Text::CollapseTree => "Contraer el árbol",
        Text::ExpandTree => "Mostrar el árbol",
        Text::MeasureTool => "Medir",
        Text::SketchDiagnostics => "Diagnóstico del boceto",
        Text::SketchDiagnosticsShort => "Diagnóstico",
        Text::PanCamera => "Desplazar: mover la cámara con el botón izquierdo",
        Text::OrbitCamera => "Rotar: girar la cámara con el botón izquierdo",
        Text::PartsFolder => "Piezas",
        Text::SubpartsFolder => "Subpiezas",
        Text::StopEditPart => "Terminar la edición de la pieza",
    }
}
