import { IDEMenuId } from '@qstudio/eventhubs';
import { ICommandBarOption, IMenuBarOptionRenderConfigStandard } from '../../../../commands/types';
import styles from './Standard.module.scss';
import { Command } from 'cmdk'

import classNames from 'classnames';
interface IStandardOptionProps {
  option: ICommandBarOption<IDEMenuId> & { renderConfig: IMenuBarOptionRenderConfigStandard };
  onSelect: (option: ICommandBarOption<IDEMenuId>) => void;
  className?: string;
}



const StandardOption: React.FC<IStandardOptionProps> = ({ option, className, onSelect }) => {

  const IconComponent = option.renderConfig.icon;
  return (
    <Command.Item className={classNames(styles.standardOption, className)} value={option.value} keywords={option.keywords} onSelect={()=>onSelect(option)}>
      {IconComponent ? <IconComponent color={option.renderConfig.iconColor} size={12} /> : null}
      <span className={styles.label}>{option.label}</span>
      {option.renderConfig.description ? <span className={styles.description}>{option.renderConfig.description}</span> : null}
      {option.shortcuts && option.shortcuts.length !== 0 ? (
        <div cmdk-vercel-shortcuts="">
          {option.shortcuts.map((key) => {
            return <kbd key={key}>{key}</kbd>
          })}
        </div>
      ) : null}
    </Command.Item>
  );
}

export {
  StandardOption,
}