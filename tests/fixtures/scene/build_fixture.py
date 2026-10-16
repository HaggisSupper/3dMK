"""Deterministic original synthetic fixture, not field data or a converter."""
import json,struct,pathlib,math
root=pathlib.Path(__file__).parent
plan=[[0,0,0],[0,-100,0],[20,-200,0]]
actual=[[0,0,0],[10,-100,0],[40,-200,10]]
tops=[[10,-150,0],[25,-150,5]]
section=[[15,0,-10],[15,-220,-10],[15,-220,30],[15,0,30]]
doc={'asset':{'version':'2.0','generator':'3DMK synthetic fixture'},'scene':0,'scenes':[{'nodes':[0,1,2,3]}],'nodes':[],'meshes':[],'materials':[],'bufferViews':[],'accessors':[],'buffers':[],'extras':{'mavisSyntheticFixture':{'name':'Planned and actual XYZ stations / top / section','fieldData':False,'units':'m','horizontalCrs':'LOCAL_TEST_METRES','verticalDatum':'collar elevation 0 m','axes':'glTF: X east, Y up, Z south','interpolation':'piecewise linear XYZ; no inferred survey or velocity model','plan':plan,'actual':actual,'topElevationMetres':-150,'expectedTopIntersections':tops,'sectionEastMetres':15,'expectedSectionIntersections':{'plan':[15,-175,0],'actual':[15,-350/3,5/3]}}}}
blob=bytearray()
def accessor(values,kind='VEC3',component=5126):
    while len(blob)%4:blob.append(0)
    offset=len(blob);flat=[x for v in values for x in v] if kind=='VEC3' else values
    blob.extend(struct.pack('<'+('f' if component==5126 else 'I')*len(flat),*flat));view=len(doc['bufferViews']);doc['bufferViews'].append({'buffer':0,'byteOffset':offset,'byteLength':len(blob)-offset})
    a={'bufferView':view,'componentType':component,'count':len(values),'type':kind}
    if kind=='VEC3':a.update(min=[min(v[i] for v in values) for i in range(3)],max=[max(v[i] for v in values) for i in range(3)])
    index=len(doc['accessors']);doc['accessors'].append(a);return index
for name,role,points,color in [('Planned well','plan',plan,[.1,.5,1,1]),('Actual well','actual',actual,[1,.5,.05,1]),('Top intersections','top',tops,[1,.1,.8,1]),('Section E=15 m','section',section,[.7,.7,.7,.25])]:
    positions=[];indices=[];mode=4
    if role in ('plan','actual'):
        for a,b in zip(points,points[1:]):
            start=len(positions);positions.extend([[a[0]-.2,a[1],a[2]],[a[0]+.2,a[1],a[2]],[b[0]+.2,b[1],b[2]],[b[0]-.2,b[1],b[2]]]);indices.extend([start,start+1,start+2,start,start+2,start+3])
    elif role=='section':positions=points;indices=[0,1,2,0,2,3]
    else:positions=points;mode=0
    primitive={'attributes':{'POSITION':accessor(positions)},'mode':mode,'material':len(doc['materials'])}
    if indices:primitive['indices']=accessor(indices,'SCALAR',5125)
    doc['materials'].append({'name':name,'pbrMetallicRoughness':{'baseColorFactor':color,'metallicFactor':0,'roughnessFactor':1},'doubleSided':True,'alphaMode':'BLEND' if color[3]<1 else 'OPAQUE'})
    doc['nodes'].append({'name':name,'mesh':len(doc['meshes']),'extras':{'id':'synthetic-'+role,'kind':role,'sourceStationsXYZ':points,'displayRepresentation':{'kind':'schematic ribbon' if role in ('plan','actual') else role,'physicalDiameter':False}}});doc['meshes'].append({'primitives':[primitive]})
doc['buffers']=[{'byteLength':len(blob)}]
raw=json.dumps(doc,separators=(',',':')).encode();raw+=b' '*((-len(raw))%4);blob+=b'\0'*((-len(blob))%4)
output=struct.pack('<III',0x46546c67,2,12+8+len(raw)+8+len(blob))+struct.pack('<II',len(raw),0x4e4f534a)+raw+struct.pack('<II',len(blob),0x004e4942)+blob
(root/'planned-actual-section.glb').write_bytes(output)
(root/'expected.json').write_text(json.dumps(doc['extras']['mavisSyntheticFixture'],indent=2)+'\n')
