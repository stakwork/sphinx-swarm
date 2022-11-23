export interface Tribes {
  name: string;
  preview: string;
  logo?: string;
  created?: string;
  pricePerMessage: number;
  amountToStake?: number;
  userCount: number;
}

export const initialTribes: Tribes[] = [
  {
    name: "Testing Sphinx",
    pricePerMessage: 0,
    preview: "https://cache.sphinx.chat/?tribe=XuOp5B9kC3CcL52svtl_LJJJFbV1OTgnq7thtOjdKJMnOuETIw_hlLkVfonozVIwz5wADlya_i946GiKFZAgMto0cDuk",
    logo: "",
    userCount: 5,
  },
  {
    name: "Sphinx Dev",
    pricePerMessage: 2,
    preview: "https://cache.sphinx.chat/?tribe=XuOp5B9kC3CcL52svtl_LJJJFbV1OTgnq7thtOjdKJMnOuETIw_hlLkVfonozVIwz5wADlya_i946GiKFZAgMto0cDuk",
    logo: "",
    userCount: 10,
  },
  {
    name: "Planet Sphinx",
    pricePerMessage: 4,
    preview: "",
    logo: "",
    userCount: 3,
  },
  {
    name: "Music",
    pricePerMessage: 2,
    preview: "",
    logo: "",
    userCount: 2,
  },
  {
    name: "Sport",
    pricePerMessage: 3,
    preview: "https://cache.sphinx.chat/?tribe=XuOp5B9kC3CcL52svtl_LJJJFbV1OTgnq7thtOjdKJMnOuETIw_hlLkVfonozVIwz5wADlya_i946GiKFZAgMto0cDuk",
    logo: "",
    userCount: 1,
  },
];
