import styles from './Welcome.module.scss';
const WelcomeDockComponent: React.FC = () => {
  return(
    <div className={styles.dockPage}>
      <div className={styles.dockPageInner}>

    <div className="flex flex-wrap gap-4">
      <button color="primary">Primary</button>
      <button color="success">Success</button>
    </div> 
        <h1>Welcome</h1>
      </div>
    </div>
  )
};

export default WelcomeDockComponent;