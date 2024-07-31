import { Tooltip, TooltipProps } from "@mantine/core";
import { TOOLTIP_COLOR } from "../../constants/style";

interface IInlineTooltipProps extends TooltipProps {
  children: React.ReactNode;
}

const InlineTooltip: React.FC<IInlineTooltipProps> = ({ children, ...props }) => {
  return (
    <Tooltip inline color={TOOLTIP_COLOR} withArrow events={{hover: true, focus: true, touch: true}} {...props}>
      {children}
    </Tooltip>
  );
}

export {
  InlineTooltip,
}