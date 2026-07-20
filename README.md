# Agentic CAD Backend

Local Rust CLI backend for heavy Computational Geometry and AI Vision tasks, designed for local execution via Mistral.rs and PDAL C++ bindings. 

## Capabilities
1. `pdf-to-3d`: Extracts wall coordinates from floorplans using a local Mistral.rs vision model and extrudes a mathematical 3D object using the `truck` CAD kernel.
2. `poisson-reconstruct`: Triggers a highly threaded KD-Tree normal estimation and Poisson wrapping via local PDAL bindings.

Outputs can be visualized in the included web viewer.