import { useMemo, useState } from 'react';
import { FloatingIndicator, UnstyledButton } from '@mantine/core';
import classes from './FloatingIndicatorMenu.module.scss';

const data = ['React', 'Vue', 'Angular', 'Svelte'];
interface IFloatingIndicatorMenuProps {
  className?: string;
  options: { label: string; value: string }[];
  value: string;
  onChange: (value: string) => void;
}
const FloatingIndicatorMenu: React.FC<IFloatingIndicatorMenuProps> = ({ className, options, value, onChange }) => {

  const optionValues = useMemo(() => options.map((option) => option.value), [options]);
  const active = useMemo(()=>optionValues.indexOf(value),[value,optionValues]);
  const [rootRef, setRootRef] = useState<HTMLDivElement | null>(null);
  const [controlsRefs, setControlsRefs] = useState<Record<string, HTMLButtonElement | null>>({});

  const setControlRef = (index: number) => (node: HTMLButtonElement) => {
    controlsRefs[index] = node;
    setControlsRefs(controlsRefs);
  };

  const controls = options.map((item, index) => (
    <UnstyledButton
      key={item.value}
      className={classes.control}
      ref={setControlRef(index)}
      onClick={() => onChange(optionValues[index])}
      mod={{ active: active === index }}
    >
      <span className={classes.controlLabel}>{item.label}</span>
    </UnstyledButton>
  ));

  return (
    <div className={classes.root} ref={setRootRef}>
      {controls}

      <FloatingIndicator
        target={controlsRefs[active]}
        parent={rootRef}
        className={classes.indicator}
      />
    </div>
  );
}

export {
  FloatingIndicatorMenu,
}