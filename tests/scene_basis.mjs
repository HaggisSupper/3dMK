import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
const THREE=await import(process.env.THREE_MODULE_PATH||'three');
const source=await fs.readFile(new URL('../public/scene-glb.js',import.meta.url),'utf8');
const moduleUrl=`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`;
const {axisBasis,convertViewBasis,objectsForGlb}=await import(moduleUrl);
assert.equal(typeof axisBasis,'function','one explicit source/display basis is required');
for(const axis of ['X','Y','Z','-X','-Y','-Z']){
 const v=new THREE.Vector3(axis.endsWith('X')?1:0,axis.endsWith('Y')?1:0,axis.endsWith('Z')?1:0).multiplyScalar(axis.startsWith('-')?-1:1);
 assert.ok(v.applyQuaternion(axisBasis(THREE,axis,'X')).distanceTo(new THREE.Vector3(1,0,0))<1e-12);
}
assert.throws(()=>axisBasis(THREE,'unknown','X'),/axis/);
const canonical={camera:{position:[1,2,3],rotation:[0,0,0,1],target:[2,3,4],projection:{type:'perspective'}},clipping:{enabled:true,planes:[{normal:[0,1,0],offset_metres:-2}]}};
const unchanged=structuredClone(canonical),display=convertViewBasis(THREE,canonical,'Y','X'),restored=convertViewBasis(THREE,display,'X','Y');
assert.deepEqual(canonical,unchanged,'conversion cannot mutate durable view');
for(const key of ['position','target','rotation'])restored.camera[key].forEach((v,i)=>assert.ok(Math.abs(v-canonical.camera[key][i])<1e-12));
assert.ok(new THREE.Vector3().fromArray(display.clipping.planes[0].normal).distanceTo(new THREE.Vector3(1,0,0))<1e-12);
assert.equal(display.clipping.planes[0].offset_metres,-2);
const workspaceSource=(await fs.readFile(new URL('../public/scene-workspace.js',import.meta.url),'utf8')).replace("'./scene-glb.js'",JSON.stringify(moduleUrl));
const {makeSceneObject,buildSceneGroup}=await import(`data:text/javascript;base64,${Buffer.from(workspaceSource).toString('base64')}`);
const template=new THREE.Group();template.sourceNodeTemplates=new Map([[0,new THREE.Mesh(new THREE.BoxGeometry(),new THREE.MeshBasicMaterial())]]);
const records=[];let id=0;
for(const sourceAxis of ['Y','Z']){
 const doc={scene:0,scenes:[{nodes:[0]}],nodes:[{mesh:0,translation:sourceAxis==='Y'?[0,5,0]:[0,0,5]}]};
 const imported=objectsForGlb(THREE,{asset_id:sourceAxis,original_name:sourceAxis+'.glb'},doc,makeSceneObject,()=>String(++id));
 imported.objects[0].source_up_axis=sourceAxis;records.push(...imported.objects);
}
const document={frame:{up_axis:'Y'},objects:Object.fromEntries(records.map(o=>[o.object_id,o]))},snapshot=structuredClone(document);
const built=await buildSceneGroup(THREE,document,async()=>template);built.group.quaternion.copy(axisBasis(THREE,'Y','X'));built.group.updateMatrixWorld(true);
for(const record of records.filter(o=>o.kind==='instance'))assert.ok(built.objects.get(record.object_id).getWorldPosition(new THREE.Vector3()).distanceTo(new THREE.Vector3(5,0,0))<1e-10);
assert.deepEqual(document,snapshot,'normalization cannot change canonical transforms');
console.log('PASS: six signed source axes to +X, canonical saved-view/clip roundtrip, mixed Y/Z file roots, immutable scene transforms');
// Source and Layout must use the same horizontal basis, not merely the same up vector.
for(const axis of ['X','Y','Z','-X','-Y','-Z']){
 const point=new THREE.Vector3(1,2,3);
 const direct=point.clone().applyQuaternion(axisBasis(THREE,axis,'X'));
 const viaCanonical=point.clone().applyQuaternion(axisBasis(THREE,axis,'Y')).applyQuaternion(axisBasis(THREE,'Y','X'));
 assert.ok(direct.distanceTo(viaCanonical)<1e-12,`${axis}: Source/Layout horizontal basis mismatch`);
 const exported=direct.clone().applyQuaternion(axisBasis(THREE,'X','Y'));
 assert.ok(exported.distanceTo(point.clone().applyQuaternion(axisBasis(THREE,axis,'Y')))<1e-12,`${axis}: derived export changed canonical orientation`);
}
console.log('PASS: all signed Source/Layout bases agree; derived exports retain prior canonical-Y orientation');
const integritySource=await fs.readFile(new URL('../public/geometry-integrity.js',import.meta.url),'utf8');
const {cropGeometry}=await import(`data:text/javascript;base64,${Buffer.from(integritySource).toString('base64')}`);
const cropSource=new THREE.BufferGeometry();cropSource.setAttribute('position',new THREE.Float32BufferAttribute([10,5,0,11,6,1],3));
const sourceToDisplay=new THREE.Matrix4().makeRotationFromQuaternion(axisBasis(THREE,'Y','X'));
const fullDisplayBounds=new THREE.Box3().setFromBufferAttribute(cropSource.attributes.position).applyMatrix4(sourceToDisplay);
const cropped=cropGeometry(THREE,cropSource,fullDisplayBounds,sourceToDisplay);
assert.ok(cropped,'full display crop must not discard all canonical points');assert.deepEqual([...cropped.attributes.position.array],[10,5,0,11,6,1]);
// An oblique display crop must test exact transformed points, not its inverse AABB.
const oblique=new THREE.Matrix4().makeRotationZ(Math.PI/4),thinBounds=new THREE.Box3(new THREE.Vector3(-.1,-2,-1),new THREE.Vector3(.1,2,1));
const cloud=new THREE.BufferGeometry();cloud.setAttribute('position',new THREE.Float32BufferAttribute([1,1,0,1,0,0,-1,-1,0],3));
const exact=cropGeometry(THREE,cloud,thinBounds,oblique);assert.deepEqual(exact.userData.sourceVertexIds,[0,2]);
console.log('PASS: full +X-display crop preserves canonical points; oblique crop uses exact source-to-display predicates');
const html=await fs.readFile(new URL('../public/index.html',import.meta.url),'utf8');
const blueprintFunction=html.match(/function applyBlueprintImage\(img\) \{[\s\S]*?\n        \}/)[0];
const boxGeometry=new THREE.BoxGeometry(2,4,6).translate(10,5,0),boxGroup=new THREE.Group();boxGroup.add(new THREE.Mesh(boxGeometry));boxGroup.quaternion.copy(axisBasis(THREE,'Y','X'));boxGroup.updateMatrixWorld(true);
const blueprintFloor=new Function('THREE','currentGroup','loadedGeometryStore',`let blueprintPlane;const currentModelMaxDim=6;const clearBlueprintPlane=()=>{};const buildSceneGraph=()=>{};const getGeometryBounds=geometries=>{const bounds=new THREE.Box3();for(const geometry of geometries){geometry.computeBoundingBox();bounds.union(geometry.boundingBox);}return bounds;};${blueprintFunction};applyBlueprintImage({width:100,height:100});return blueprintPlane.position.y;`)(THREE,boxGroup,[boxGeometry]);
assert.equal(blueprintFloor,3,'blueprint floor is source-local, not display Y');
for(const name of ['exportDXF','exportFloorPlanPDF','exportElevationsPDF'])assert.match(html,new RegExp(`function ${name}\\(\\) \\{\\s*if\\(sceneWorkspace\\?\\.active\\)`),'source-only drawing action must guard layout mode');
assert.match(html,/const m = exportBasis\.clone\(\)\.multiply\(child\.matrixWorld\)/,'DXF removes display basis before its canonical-Y slice');
console.log('PASS: blueprint source-local floor, source-only drawing guards and canonical DXF transform');

for(const axis of ['x','y','z','-x','-y','-z'])assert.ok(axisBasis(THREE,axis,'x').angleTo(axisBasis(THREE,axis.toUpperCase(),'X'))<1e-12,'Rust lowercase axis wire format');
const wireRoot=objectsForGlb(THREE,{asset_id:'wire',original_name:'wire.glb'},{scene:0,scenes:[{nodes:[]}],nodes:[]},makeSceneObject,()=> 'wire').objects[0];
assert.equal(wireRoot.source_up_axis,'y','new source metadata must serialize to Rust UpAxis');
console.log('PASS: Rust lowercase up-axis wire format and new source metadata');
