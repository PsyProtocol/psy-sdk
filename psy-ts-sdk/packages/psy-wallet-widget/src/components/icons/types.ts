interface IIconAltProps {
    color?: string;
    size?: number;
}

type TSVGIconProps = React.HTMLProps<SVGSVGElement> & IIconAltProps;

export type { TSVGIconProps };
