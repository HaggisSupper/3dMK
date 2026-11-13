// Presentation-only orthonormal basis. Never apply this to immutable vertices.
export function axisBasis(THREE, source='Y', target='X') {
    // A fixed canonical-Y basis preserves the established horizontal orientation.
    // Independently choosing shortest arcs between each pair would introduce roll.
    const toCanonical=axis=>{
        const q=new THREE.Quaternion();
        switch(typeof axis==='string'?axis.toUpperCase():axis){
            case 'Y': return q;
            case 'Z': return q.setFromAxisAngle(new THREE.Vector3(1,0,0),-Math.PI/2);
            case '-Z': return q.setFromAxisAngle(new THREE.Vector3(1,0,0),Math.PI/2);
            case 'X': return q.setFromAxisAngle(new THREE.Vector3(0,0,1),Math.PI/2);
            case '-X': return q.setFromAxisAngle(new THREE.Vector3(0,0,1),-Math.PI/2);
            case '-Y': return q.setFromAxisAngle(new THREE.Vector3(1,0,0),Math.PI);
            default: throw new Error(`Unsupported up-axis: ${axis}`);
        }
    };
    return toCanonical(target).invert().multiply(toCanonical(source));
}
export function convertViewBasis(THREE, view, source='Y', target='X') {
    const result=structuredClone(view),q=axisBasis(THREE,source,target);
    for(const key of ['position','target'])if(result.camera?.[key])result.camera[key]=new THREE.Vector3().fromArray(result.camera[key]).applyQuaternion(q).toArray();
    if(result.camera?.rotation)result.camera.rotation=q.clone().multiply(new THREE.Quaternion().fromArray(result.camera.rotation)).toArray();
    for(const plane of result.clipping?.planes||[])plane.normal=new THREE.Vector3().fromArray(plane.normal).applyQuaternion(q).toArray();
    return result;
}

// Immutable GLB source hierarchy -> editable 3DMK placement records.
// Source node selectors address own primitives only; children remain separate records.
export function glbDocument(bytes) {
    const view=new DataView(bytes);
    if(view.byteLength<20||view.getUint32(0,true)!==0x46546c67||view.getUint32(4,true)!==2||view.getUint32(8,true)!==view.byteLength||view.getUint32(16,true)!==0x4e4f534a)throw new Error('Expected an embedded glTF 2 GLB');
    const length=view.getUint32(12,true);if(20+length>view.byteLength)throw new Error('Truncated GLB JSON');
    return JSON.parse(new TextDecoder().decode(new Uint8Array(bytes,20,length)));
}

export function glbFrame(document) {
    const mapping=document.extras?.mavisExchange?.mapping;
    if(!mapping)return {translation:[0,0,0],crs:null,registered:false};
    if(!Array.isArray(mapping.origin)||mapping.origin.length!==3||!mapping.origin.every(Number.isFinite))throw new Error('GLB exchange origin is invalid');
    if(mapping.gltf_axes!=='east,up,-north; metres')throw new Error('Unsupported GLB exchange coordinate axes; registration is required');
    return {translation:[mapping.origin[0],mapping.origin[2],-mapping.origin[1]],crs:mapping.crs||null,registered:false};
}

function nodeTransform(THREE,node) {
    const p=new THREE.Vector3(),q=new THREE.Quaternion(),s=new THREE.Vector3(1,1,1);
    if(node.matrix){
        if(node.matrix.length!==16||!node.matrix.every(Number.isFinite))throw new Error('Invalid GLB node matrix');
        const original=new THREE.Matrix4().fromArray(node.matrix);original.decompose(p,q,s);
        const restored=new THREE.Matrix4().compose(p,q,s);
        if(original.elements.some((v,i)=>Math.abs(v-restored.elements[i])>1e-8*Math.max(1,Math.abs(v))))throw new Error('Sheared node matrices cannot be represented safely by the scene transform editor');
    }else {p.fromArray(node.translation||[0,0,0]);q.fromArray(node.rotation||[0,0,0,1]);s.fromArray(node.scale||[1,1,1]);}
    return {translation:p.toArray(),rotation:q.toArray(),scale:s.toArray()};
}

export function objectsForGlb(THREE,asset,document,makeObject,newId=()=>crypto.randomUUID(),rootName=asset.original_name) {
    const sourceNodes=document.nodes||[],scene=document.scenes?.[document.scene??0];
    if(!scene||!Array.isArray(scene.nodes))throw new Error('GLB has no default scene');
    const root=makeObject(newId(),String(rootName).slice(0,160)),objects=[root],seen=new Set(),frame=glbFrame(document);
    root.transform.translation=frame.translation;
    root.source_up_axis='y'; // glTF 2 defines +Y up; nonstandard sources need an explicit override.
    const visit=(index,parent,depth)=>{
        if(depth>60||objects.length>4090)throw new Error('GLB scene exceeds the bounded hierarchy size');
        if(!Number.isInteger(index)||!sourceNodes[index]||seen.has(index))throw new Error('GLB scene has a missing, cyclic or multiply-parented node');
        seen.add(index);const node=sourceNodes[index];
        if(node.skin!==undefined||node.weights||node.extensions?.EXT_mesh_gpu_instancing)throw new Error('Skinned, morphed or GPU-instanced nodes need a supported source workflow');
        const group=makeObject(newId(),node.name||node.extras?.id||`Node ${index}`);group.name=String(group.name).slice(0,160);group.parent_id=parent;group.transform=nodeTransform(THREE,node);
        const role=node.extras?.kind;if(['plan','actual','top','section','slice'].includes(role))group.role=role;
        objects.push(group);
        if(node.mesh!==undefined){const instance=makeObject(newId(),'Geometry',asset.asset_id,group.role);instance.parent_id=group.object_id;instance.asset_node_index=index;objects.push(instance);}
        for(const child of node.children||[])visit(child,group.object_id,depth+1);
    };
    for(const index of scene.nodes)visit(index,root.object_id,0);
    return {objects,root_id:root.object_id,frame};
}

export async function glbSceneTemplate(THREE,gltf,document) {
    const group=new THREE.Group(),nodeTemplates=new Map(),frame=glbFrame(document);
    const reachable=new Set(),visit=index=>{if(reachable.has(index))throw new Error('Invalid source node hierarchy');reachable.add(index);for(const child of document.nodes[index].children||[])visit(child);};
    for(const root of document.scenes?.[document.scene??0]?.nodes||[])visit(root);
    for(const index of reachable){const node=document.nodes[index];if(node.mesh===undefined)continue;
        const template=(await gltf.parser.getDependency('mesh',node.mesh)).clone(true);
        template.position.set(0,0,0);template.quaternion.identity();template.scale.set(1,1,1);template.updateMatrix();
        template.traverse(o=>{if(o.isSkinnedMesh||Object.keys(o.geometry?.morphAttributes||{}).length)throw new Error('Skinned or morph geometry is unsupported in scene layout');});
        nodeTemplates.set(index,template);
    }
    // Preserve the legacy whole-asset instance path for already-saved scenes.
    group.add(gltf.scene);group.position.fromArray(frame.translation);
    group.userData.sourceDocument=document;group.userData.sourceFrame=frame;
    // Maps are deliberately outside userData: Three.clone JSON-copies userData.
    group.sourceNodeTemplates=nodeTemplates;
    return group;
}
