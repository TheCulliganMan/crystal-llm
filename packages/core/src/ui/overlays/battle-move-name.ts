import { MoveName } from '../../core/enums';

// ASM: move name formatting is a direct underscore-to-space transform.
export const formatMoveName = (moveName: MoveName): string => {
  return moveName.replace(/_/g, ' ');
};
