import { useEffect, useMemo, useRef, useState } from 'react';
import { EventHub, seq, uuidv4 } from '@qstudio/utils';
import { useRenamableFile } from '../../hooks/useRenamableFile';
import styles from './Stage.module.scss';
import { EditorUIEventType, IDEMenuId, ProjectFilesEvent, ProjectFilesEventType, SplitPanelsEventType } from '@qstudio/eventhubs';
import { ISyncFileStore } from '@qstudio/storage';
import { IDEContext } from '../../utils/ideContext';
import { getFileExtForFileName, getLanguageForFilePath } from '../../utils/fileIcons';
import QVizRenderer from '../../qviz/QVizRenderer';
import {makeElemAttributes, makeSVGElemAttributes} from '@qstudio/qsvg';
import { ITreeJunctionLayout, NodeAnchor, QSceneManager, QWProof, QWRect, QWTreeJunction, RectSide, simpleDebugTree } from '@qstudio/core';
import {CityBlockSceneManager, EXAMPLE_SCENARIO, EXAMPLE_SCENARIO_2} from '@qstudio/qviz-city';
import { setupWidgetStyles } from '../../qviz/widgetStyles';
import BlockVizDockComponent from '../../components/BlockViz';
interface IStageDockComponentProps {
  filePath: string;
  fileEventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  fileStore: ISyncFileStore;
  ctx: IDEContext;
}
const StageDockComponent: React.FC<IStageDockComponentProps> = ({ filePath, fileEventHub, fileStore, ctx }) => {

  return <BlockVizDockComponent ctx={ctx} />;
};

export default StageDockComponent;