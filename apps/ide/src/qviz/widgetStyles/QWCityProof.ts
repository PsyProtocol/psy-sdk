import styles from './QWCityProof.module.scss';
import { IQVCityProofStyleDef } from "@qstudio/qviz-city";

const qwCityProofStyleDef: IQVCityProofStyleDef = {
  states: {
    hidden: styles.hidden,
    waiting: styles.waiting,
    proving: styles.proving,
    proved: styles.proved,
  },
  refLink: styles.refLink,
  base: styles.qwCityProofWidget,
  outerGroup: styles.outerGroup,
  borderRect: styles.borderRect,
  label: styles.label,
  statusGroup: styles.statusGroup,
  statusRect: styles.statusRect,
  statusText: styles.statusText,
  styleConfig: {
    fillColorClass: styles.iconFillColor,
    strokeColorClass: styles.iconStrokeColor,
    gClass: styles.iconGroup,
  },
  iconRoot: styles.iconRoot,
};

export {
  qwCityProofStyleDef,
}