// Only stdin commands; no dataset/path/label files or evaluator imports.
import readline from 'node:readline';
import {VERSION,exact,fit,predict,hash,need} from './learner-core.mjs';
let state=null;
const input=readline.createInterface({input:process.stdin,crlfDelay:Infinity});
for await(const line of input){const start=process.hrtime.bigint();let out;try{const req=JSON.parse(line);
 if(req.op==='fit'){need(!state,'fit:already-fitted');const fitted=fit(req);state=fitted;out={status:'fitted',version:VERSION,...state,fit_steps:state.model.settings.steps};}
 else if(req.op==='predict')out=predict(req,state);
 else if(req.op==='inspect'){exact(req,['op','version'],'inspect');need(req.version===VERSION&&state,'inspect:state');out={status:'frozen',version:VERSION,model_sha256:hash(state.model),model:state.model};}
 else throw Error('op:unsupported');
 }catch(error){out={status:'rejected',error:error.message};}
 console.log(JSON.stringify(out));
 console.error(JSON.stringify({pid:process.pid,operation:out.status,wall_at:new Date().toISOString(),compute_ns:String(process.hrtime.bigint()-start),rss_bytes:process.memoryUsage().rss}));
}
