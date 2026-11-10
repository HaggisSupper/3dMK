import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
const THREE=await import(process.env.THREE_MODULE_PATH||'three');
const source=await fs.readFile(new URL('../public/scene-glb.js',import.meta.url),'utf8');
const moduleUrl=`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`;
const {objectsForGlb,glbDocument,glbSceneTemplate}=await import(moduleUrl);
const workspaceSource=(await fs.readFile(new URL('../public/scene-workspace.js',import.meta.url),'utf8')).replace("'./scene-glb.js'",JSON.stringify(moduleUrl));
const {makeSceneObject,buildSceneGroup}=await import(`data:text/javascript;base64,${Buffer.from(workspaceSource).toString('base64')}`);
const loaderPath=process.env.GLTF_LOADER_PATH||'three/addons/loaders/GLTFLoader.js';
const {GLTFLoader}=await import(loaderPath);globalThis.ProgressEvent??=class{constructor(type,options={}){this.type=type;Object.assign(this,options);}};
const bytes=await fs.readFile(new URL('fixtures/scene/planned-actual-section.glb',import.meta.url));const buffer=bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength),document=glbDocument(buffer);
const parsed=await new GLTFLoader().parseAsync(buffer,'');const template=await glbSceneTemplate(THREE,parsed,document);
let sequence=0;const newId=()=>`id-${++sequence}`,asset={asset_id:'source-one',original_name:'same.glb'},other={asset_id:'source-two',original_name:'same.glb'};
const one=objectsForGlb(THREE,asset,document,makeSceneObject,newId),two=objectsForGlb(THREE,other,document,makeSceneObject,newId);
const objects=Object.fromEntries([...one.objects,...two.objects].map(o=>[o.object_id,o]));assert.equal(Object.keys(objects).length,one.objects.length+two.objects.length,'same file/name cannot collide');
assert.equal(Object.values(objects).filter(o=>o.parent_id===null).length,2);assert.deepEqual(one.objects.filter(o=>o.asset_node_index!==undefined).map(o=>o.asset_node_index),[0,1,2,3]);
let built=await buildSceneGroup(THREE,{objects},async()=>template);assert.equal(built.group.children.length,2);
let primitives=0;built.group.traverse(o=>{if(o.isMesh||o.isPoints)primitives++;});assert.equal(primitives,8,'each primitive rendered once per file');
const roots=Object.values(objects).filter(o=>!o.parent_id);roots[0].visible=false;
built=await buildSceneGroup(THREE,{objects},async()=>template);assert.equal(built.objects.get(roots[0].object_id).visible,false);assert.equal(built.objects.get(roots[1].object_id).visible,true);
const saved=JSON.parse(JSON.stringify({objects}));assert.deepEqual(saved,{objects});const again=await buildSceneGroup(THREE,saved,async()=>template);assert.equal(again.objects.size,built.objects.size);
// Parent transform, non-uniform scale and negative scale retain exact hierarchy.
const nested={scene:0,scenes:[{nodes:[0]}],nodes:[{name:'parent',translation:[20.01,0,0],scale:[-2,3,1],children:[1]},{name:'child',mesh:0,translation:[.01,-2,4]}]};
const nestedRecords=objectsForGlb(THREE,asset,nested,makeSceneObject,newId);const leaf=nestedRecords.objects.find(o=>o.asset_node_index===1);const mesh=new THREE.Mesh(new THREE.BoxGeometry(),new THREE.MeshStandardMaterial());const nodeTemplate=new THREE.Group();nodeTemplate.sourceNodeTemplates=new Map([[1,mesh]]);
const nestedBuilt=await buildSceneGroup(THREE,{objects:Object.fromEntries(nestedRecords.objects.map(o=>[o.object_id,o]))},async()=>nodeTemplate);
const world=nestedBuilt.objects.get(leaf.object_id).getWorldPosition(new THREE.Vector3());assert.ok(world.distanceTo(new THREE.Vector3(19.99,-6,4))<1e-10);
const mapped=structuredClone(nested);mapped.extras={mavisExchange:{mapping:{origin:[150,-120,2.5],crs:'LOCAL:TEST',gltf_axes:'east,up,-north; metres'}}};const mappedRecords=objectsForGlb(THREE,asset,mapped,makeSceneObject,newId);assert.deepEqual(mappedRecords.objects[0].transform.translation,[150,2.5,120]);
const cyclic=structuredClone(nested);cyclic.nodes[1].children=[0];assert.throws(()=>objectsForGlb(THREE,asset,cyclic,makeSceneObject,newId),/cyclic/);
const shear=structuredClone(nested);shear.nodes[0]={matrix:[1,0,0,0,.5,1,0,0,0,0,1,0,0,0,0,1]};assert.throws(()=>objectsForGlb(THREE,asset,shear,makeSceneObject,newId),/Sheared/);
console.log('PASS: multi-GLB unique roots, source-node primitive fidelity, parent TRS, subtree visibility, persistence, declared origin mapping and fail-closed invalid hierarchies');

// Optional externally delivered original synthetic engineering examples.
if(process.env.GEOSPATIAL_FIXTURE_DIR){
 for(const name of ['well-a-adapter.glb','well-b-adapter.glb','synthetic-block-preview.glb','bha-dimensional-preview.glb']){
  const data=await fs.readFile(`${process.env.GEOSPATIAL_FIXTURE_DIR}/${name}`),array=data.buffer.slice(data.byteOffset,data.byteOffset+data.byteLength),doc=glbDocument(array),gltf=await new GLTFLoader().parseAsync(array,''),base=await glbSceneTemplate(THREE,gltf,doc),records=objectsForGlb(THREE,{asset_id:name,original_name:name},doc,makeSceneObject,newId);
  const result=await buildSceneGroup(THREE,{objects:Object.fromEntries(records.objects.map(o=>[o.object_id,o]))},async()=>base);assert.equal(result.group.children.length,1);console.log(`PASS fixture: ${name}, ${records.objects.length} scene records, ${base.sourceNodeTemplates.size} source mesh nodes`);
 }
}
