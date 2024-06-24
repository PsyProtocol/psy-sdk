import LazyLoadAnimation from './LazyLoadAnimation';
import styles from './LazyLoader.module.scss';

const LazyLoader: React.FC = () => {
  return(
    <div className={styles.lazyLoader}>
      <LazyLoadAnimation width={48} height={48} className={styles.loadingAnimation} />
      <div className={styles.loadingMessage}>
        Loading Editor...
      </div>
    </div>
  )
};

export default LazyLoader;