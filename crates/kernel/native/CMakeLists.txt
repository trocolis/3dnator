cmake_minimum_required(VERSION 3.20)
project(nator_occt LANGUAGES CXX)

# A ponte usa a API OCCT 7.x. A migração para 8.x precisa de validação própria.
# O OpenCASCADEConfig.cmake inclui os alvos na ordem pedida e verifica dependências a cada
# inclusão: Visualization precisa vir antes de ApplicationFramework e DataExchange.
find_package(OpenCASCADE REQUIRED COMPONENTS FoundationClasses ModelingData ModelingAlgorithms Visualization ApplicationFramework DataExchange)
if(DEFINED OpenCASCADE_MAJOR_VERSION AND NOT OpenCASCADE_MAJOR_VERSION EQUAL 7)
  message(FATAL_ERROR "Este protótipo requer OCCT 7.x (alvo: 7.6 a 7.9).")
endif()

add_library(nator_occt STATIC bridge.cpp)
target_compile_features(nator_occt PRIVATE cxx_std_17)
if(MSVC)
  target_compile_options(nator_occt PRIVATE /utf-8 /EHsc)
endif()
target_include_directories(nator_occt SYSTEM PRIVATE ${OpenCASCADE_INCLUDE_DIR})
set_target_properties(nator_occt PROPERTIES
  ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/lib"
  ARCHIVE_OUTPUT_DIRECTORY_RELEASE "${CMAKE_BINARY_DIR}/lib"
  POSITION_INDEPENDENT_CODE ON)

set(nator_libraries TKernel TKMath TKG2d TKG3d TKGeomBase TKBRep TKGeomAlgo TKTopAlgo TKPrim TKBO TKBool TKFillet TKOffset TKMesh TKHLR TKXSBase TKCDF TKLCAF TKCAF TKXCAF)
# Os toolkits STEP foram reorganizados no OCCT 7.8.
find_library(nator_step NAMES TKDESTEP HINTS ${OpenCASCADE_LIBRARY_DIR})
if(nator_step)
  list(APPEND nator_libraries TKDESTEP)
else()
  list(APPEND nator_libraries TKSTEP TKSTEPBase TKSTEPAttr TKXDESTEP)
endif()

file(WRITE "${CMAKE_BINARY_DIR}/nator-links.txt" "")
foreach(name IN LISTS nator_libraries)
  find_library(nator_${name} NAMES ${name} HINTS ${OpenCASCADE_LIBRARY_DIR} REQUIRED)
  get_filename_component(library_dir "${nator_${name}}" DIRECTORY)
  file(APPEND "${CMAKE_BINARY_DIR}/nator-links.txt" "path=${library_dir}\nlib=${name}\n")
  target_link_libraries(nator_occt PRIVATE "${nator_${name}}")
endforeach()
