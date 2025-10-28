import styled from 'styled-components';

const ADDRESS_MODAL_PADDING = '16px';
const ADDRESS_MODAL_PADDING_DOUBLE = '32px';

export const AddressModalContainer = styled.div`
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 99999;
  display: flex;
  padding: 48px ${ADDRESS_MODAL_PADDING} ${ADDRESS_MODAL_PADDING} ${ADDRESS_MODAL_PADDING};
  overflow: hidden;
  background-color: #111;
  align-items: center;
  justify-content: flex-start;
  flex-direction: column;
`;

export const AddressModalInner = styled.div`
  position: relative;
  top: 0;
  left: 0;
  padding: ${ADDRESS_MODAL_PADDING} ${ADDRESS_MODAL_PADDING} ${ADDRESS_MODAL_PADDING} ${ADDRESS_MODAL_PADDING};
  width: 100%;
  display: block;
  max-width: 700px;
  max-height: 800px;
  background-color: #222;
  overflow: hidden;
`;

export const ModalTop = styled.div`
  position: absolute;
  top: 0px;
  left: ${ADDRESS_MODAL_PADDING};
  padding: 32px ${ADDRESS_MODAL_PADDING} ${ADDRESS_MODAL_PADDING} 0px;
  height: 32px;
  width: calc(100% - 16px);
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
  font-size: 24px;
  z-index: 88;
`;

export const ModalContent = styled.div`
  overflow-y: auto;
  overflow-x: hidden;
  position: relative;
  top: 0px;
  left: 0;
  width: 100%;
  padding: 32px 0px 0px 0px;
`;

export const AddressModalTitle = styled.div`
  font-family: var(--wallet-widget-font-sans);
  font-size: 24px;
  font-weight: 300;
  color: #fff;
`;