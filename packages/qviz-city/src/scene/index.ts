import { IQSRenderContext, IQWTreeJunctionConfig, ITreeJunctionLayout, QEDVizPaper, QSceneManager, QWidget, RectSide } from "@qstudio/core";
import { ICSProofNode, ISimpleCityBlock } from "../scenario/types";
import { ProofStateManager, ProofTreeManager, genProofTreeWidgets } from "../scenario/proofTree";
import { ICitySighashGroth16ProofResult } from "@qstudio/city-block";
import { QWCityBlockGroup } from "../widgets/CityBlockGroup";
const baseLayout: ITreeJunctionLayout = {
  direction: RectSide.Bottom,
  siblingSpacing: 30,
  levelSpacing: 20,
  parentAnchor: RectSide.Bottom,
  childAnchor: RectSide.Top,
  edgeClassName: "qv-simple-edge-1",
};
const treeConfig: IQWTreeJunctionConfig = {
  layout: baseLayout,
}
function createSighashProofTree(stateManager: ProofStateManager, config: ICitySighashGroth16ProofResult, treeJunctionConfig: IQWTreeJunctionConfig){
  const lower: ICSProofNode = {
    id: config.sighash_final,
    dependencies: [
      {id: config.sighash_introspection, dependencies: []},
      {id: config.state_transition_reference, dependencies: [], is_ref: true},
    ]
  };
  const root: ICSProofNode = {
    id: config.groth16_final,
    dependencies: [lower],
  };

  const sighashProofTree = stateManager.addProofTree();
  sighashProofTree.addProofWidgetRoot(root, treeJunctionConfig);
  return sighashProofTree;
}
class CityBlockSceneManager {
  qscene: QSceneManager;
  scenario: ISimpleCityBlock;
  stateManager: ProofStateManager;
  stateTransitionTree: ProofTreeManager;
  sighashProofs: ProofTreeManager[];
  blockGroup: QWCityBlockGroup;
  constructor(qscene: QSceneManager, scenario: ISimpleCityBlock) {
    this.qscene = qscene;
    this.scenario = scenario;
    this.stateManager = new ProofStateManager();
    this.stateTransitionTree = genProofTreeWidgets(this.stateManager, scenario.stateTransitionRoot, treeConfig, qscene.getRenderContext());
    this.sighashProofs = scenario.sighashProofs.map(x=>createSighashProofTree(this.stateManager, x, treeConfig));

    const blockGroup = QWCityBlockGroup.create({
      stateTransitionGroup: this.stateTransitionTree.getRootWidget(),
      sighashGroups: this.sighashProofs.map(x=>x.getRootWidget()),
    });
    this.blockGroup = blockGroup;
  }

  setVizPaper(vizPaper: QEDVizPaper) {
    this.qscene.setVizPaper(vizPaper);
  }

  getRootWidget() {
    return this.blockGroup;
  }

  async proveNextJobs(context: IQSRenderContext, duration = 1000) {
    Promise.all([
      this.stateTransitionTree,
      ...this.sighashProofs,
    ].map(x=>x.proveNextJobs(context, duration)));
  }
}

export {
  CityBlockSceneManager,
}