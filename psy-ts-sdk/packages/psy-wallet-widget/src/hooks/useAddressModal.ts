import { create } from "zustand";

enum AddressModalType {
    Closed = 0,
    Import = 1,
    // ClaimDeposit = 2,
    Transfer = 3,
    // Withdraw = 4,
    ExportPrivateKey = 5,
}
type OnCompleteListener = (resultData: any, activeModalData: any, activeModal: AddressModalType) => void;
type OnCancelListener = (activeModalData: any, activeModal: AddressModalType) => void;
interface IAddressModalEventListeners {
    onComplete?: OnCompleteListener;
    onCancel?: OnCancelListener;
}
interface IAddressModalState {
    activeModalType: AddressModalType;
    activeModalData: any;
    eventListeners: IAddressModalEventListeners;
    openModal: (
        modalType: AddressModalType,
        activeModalData?: any,
        eventListeners?: IAddressModalEventListeners
    ) => any;
    cancelModal: () => any;
    completeModal: (
        resultData?: any,
        newModalType?: AddressModalType,
        newActiveModalData?: any,
        eventListeners?: IAddressModalEventListeners
    ) => any;
}
const useAddressModal = create<IAddressModalState>((set, get) => ({
    activeModalType: AddressModalType.Closed,
    activeModalData: null,
    eventListeners: {},

    openModal: (modalType: AddressModalType, activeModalData?: any, eventListeners: IAddressModalEventListeners = {}) =>
        set((state) => {
            const currentState = get();
            if (currentState.activeModalType === modalType) {
                return { eventListeners };
            } else if (currentState.activeModalType !== AddressModalType.Closed) {
                const onCancel = currentState.eventListeners.onCancel;
                if (onCancel) {
                    onCancel(currentState.activeModalData, currentState.activeModalType);
                }
            }
            return { activeModalType: modalType, eventListeners };
        }),
    cancelModal: () =>
        set((state) => {
            const currentState = get();
            if (currentState.activeModalType === AddressModalType.Closed) {
                return {};
            } else {
                const onCancel = currentState.eventListeners.onCancel;
                if (onCancel) {
                    onCancel(currentState.activeModalData, currentState.activeModalType);
                }
                return { activeModalType: AddressModalType.Closed, eventListeners: {} };
            }
        }),
    completeModal: (
        resultData?: any,
        newModalType: AddressModalType = AddressModalType.Closed,
        newActiveModalData: any = {},
        eventListeners: IAddressModalEventListeners = {}
    ) =>
        set((state) => {
            const currentState = get();
            if (currentState.activeModalType === AddressModalType.Closed) {
                return {};
            } else {
                const onComplete = currentState.eventListeners.onComplete;
                if (onComplete) {
                    onComplete(resultData, currentState.activeModalData, currentState.activeModalType);
                }
            }
            if (newModalType === AddressModalType.Closed) {
                return { activeModalType: AddressModalType.Closed, eventListeners: {} };
            } else {
                return { activeModalType: newModalType, eventListeners, activeModalData: newActiveModalData };
            }
        }),
}));
export { useAddressModal, AddressModalType };
