import { IQSRenderContext, IQWTreeJunctionConfig, QWTreeJunction, QWidget } from "@qstudio/core";
import { ICSProofNode, ISimpleCityBlock } from "./types";
import { CityProofStateType, IQWCityProofState, QWCityProof } from "../widgets";
import { deserializeJobId } from "@qstudio/city-block";
function waitMs(duration: number){
  return new Promise((resolve)=>{
    setTimeout(resolve, duration);
  });
}

class ProofStateManager {
  completed: Record<string, boolean> = {};
  proofTrees: ProofTreeManager[] = [];

  clearCompleted(){
    Object.keys(this.completed).forEach(id=>{
      this.proofTrees.forEach(pt=>pt.updateProofWidgetState(id, {stateType: CityProofStateType.Waiting}))
    });
    this.completed = {};
  }
  setCompleted(jobId: string, completed: boolean){
    this.completed[jobId] = completed;
  }
  isCompleted(jobId: string){
    return !!this.completed[jobId];
  }
  addProofTree(): ProofTreeManager {
    const newTree = new ProofTreeManager(this);
    this.proofTrees.push(newTree);
    return newTree;
  }
  getSourceProofWidget(id: string){
    for(let i=0;i<this.proofTrees.length;i++){
      const widget = this.proofTrees[i].getSourceProofWidget(id);
      if(widget){
        return widget;
      }
    }
    return null;
  }

}
class ProofTreeManager {
  junctions: Record<string, QWTreeJunction> = {};
  dependencies: Record<string, string[]> = {};
  dependants: Record<string, string[]> = {};
  proofWidgets: Record<string, QWCityProof> = {};
  rootId: string = "";
  stateManager: ProofStateManager;
  refJobIds: Record<string, boolean> = {};
  constructor(stateManager: ProofStateManager){
    this.stateManager = stateManager;
  }
  getSourceProofWidget(source: string){
    if(this.refJobIds[source]){
      return null;
    }
    return this.proofWidgets[source];
  }
  addProofWidget(node: ICSProofNode, treeConfig: IQWTreeJunctionConfig): QWidget<any, any, any>{
    if(node.is_ref === true){
      this.refJobIds[node.id] = true;
    }
    this.dependants[node.id] = this.dependants[node.id] || [];
    const widget = new QWCityProof({jobId: node.id, isRef: !!node.is_ref});
    this.proofWidgets[node.id] = widget;
    this.dependencies[node.id] = node.dependencies.map(x=>x.id);
    if(node.dependencies.length === 0){
      return widget;
    }
    const childWidgets = node.dependencies.map(x=>{
      if(this.dependants[x.id]){
        this.dependants[x.id].push(node.id);
      }else{
        this.dependants[x.id] = [node.id];
      }
      return this.addProofWidget(x, treeConfig);
    });
    const junction = QWTreeJunction.create(widget, childWidgets, treeConfig);
    this.junctions[node.id] = junction;
    return junction;
  }
  updateProofWidgetState(jobId: string, stateUpdate: Partial<IQWCityProofState>){
    if(this.proofWidgets[jobId]){
      this.proofWidgets[jobId].updateState(stateUpdate);
    }
  }
  addProofWidgetRoot(node: ICSProofNode, treeConfig: IQWTreeJunctionConfig){
    this.rootId = node.id;
    return this.addProofWidget(node, treeConfig);
  }
  getRootWidget(){
    return this.junctions[this.rootId] || this.proofWidgets[this.rootId];
  }
  canProveJob(jobId: string){
    if(this.refJobIds[jobId]){
      return this.stateManager.isCompleted(jobId);
    }
    const deps = this.dependencies[jobId];
    return deps.length === 0 || deps.every(x=>this.stateManager.isCompleted(x));
  }
  getCurrentJobIds(){
    return Object.keys(this.proofWidgets).filter(x=>!this.stateManager.isCompleted(x) && this.canProveJob(x));
  }
  consumeChildren(context: IQSRenderContext, jobId: string, duration: number){
    const deps = this.dependencies[jobId];
    deps.forEach(x=>this.consumeRef(context, x, jobId, duration));
  }
  hasProofWidget(jobId: string): boolean {
    return Object.hasOwnProperty.call(this.proofWidgets, jobId) && (!!this.proofWidgets[jobId]);
  }
  hasSourceProofWidget(jobId: string): boolean {
    return this.hasProofWidget(jobId) && !this.refJobIds[jobId];
  }
  consumeRef(context: IQSRenderContext, ref: string, destination: string, duration: number){
    const refWidget = this.hasSourceProofWidget(ref) ? this.proofWidgets[ref] : this.stateManager.getSourceProofWidget(ref);
    if(!refWidget){
      return false;
    }
    const destinationWidget = this.proofWidgets[destination];
    destinationWidget.consume(context, refWidget, duration);
    return true;
  }
  async proveNextJobs(context: IQSRenderContext, duration: number){
    const currentJobIds = this.getCurrentJobIds();
    const widgets = currentJobIds.map(x=>this.proofWidgets[x]);
    widgets.forEach(x=>{
      x.updateState({stateType: CityProofStateType.Proving});
    });
    currentJobIds.forEach(x=>{
      this.consumeChildren(context, x, duration);
    });
    
    await waitMs(duration);
    widgets.forEach(x=>{
      x.updateState({stateType: CityProofStateType.Proved});
    });

    currentJobIds.forEach(x=>{
      if(!this.refJobIds[x]){
        this.stateManager.setCompleted(x, true);
      }
    });
    return currentJobIds;
  }
}
function genProofTreeWidgets(stateManager: ProofStateManager, root: ICSProofNode, treeConfig: IQWTreeJunctionConfig, context: IQSRenderContext) :  ProofTreeManager {
  const manager = stateManager.addProofTree();
  manager.addProofWidgetRoot(root, treeConfig);
  return manager;
}

export {
  genProofTreeWidgets,
  ProofTreeManager,
  ProofStateManager,
}