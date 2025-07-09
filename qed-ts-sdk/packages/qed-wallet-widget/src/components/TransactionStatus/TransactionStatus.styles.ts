import styled, { keyframes, css } from 'styled-components';

const spin = keyframes`
  0% { 
    transform: rotate(0deg);
    stroke-dashoffset: 66;
  } 
  50% {
    transform: rotate(540deg);
    stroke-dashoffset: 314;
  } 
  100% {
    transform: rotate(1080deg);
    stroke-dashoffset: 66;
  }
`;

const draw = keyframes`
  to {
    stroke-dashoffset: 0;
  }
`;

interface StatusSVGProps {
  loading: boolean;
}

export const StatusSVG = styled.svg<StatusSVGProps>`
  .tick {
    stroke: #63bc01;
    stroke-width: 6;
    transition: all 1s;
    ${props => props.loading && css`
      opacity: 0;
    `}
    ${props => !props.loading && css`
      stroke-dasharray: 1000;
      stroke-dashoffset: 1000;
      animation: ${draw} 8s ease-out forwards;
    `}
  }

  .circle {
    stroke: #63bc01;
    stroke-width: 6;
    transform-origin: 50px 50px 0;
    transition: all 1s;
    stroke-dasharray: 500;

    ${props => props.loading && css`
      stroke: #4c4c4c;
      stroke-dasharray: 314;
      stroke-dashoffset: 1000;
      animation: ${spin} 3s linear infinite;
    `}

    ${props => !props.loading && css`
      stroke-dashoffset: 66;
      stroke: #63bc01;
    `}
  }
`;