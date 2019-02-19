import { LitElement, html, customElement, property } from 'lit-element';
import {
  Branch,
  Context,
  Commit,
  ContextHistory,
  ChildrenCommit
} from '../types';
import { connect } from 'pwa-helpers/connect-mixin';
import { store, RootState } from '../../store';
import { selectVersionControl, selectContextHistory } from '../state/selectors';
import { getContextHistory } from '../state/actions';

import 'gitgraph.js';

@customElement('commit-history')
export class CommitHistory extends connect(store)(LitElement) {
  @property({ type: Object })
  public context: Context;

  @property({ type: Array })
  public branches: Branch[];

  @property({ type: Boolean })
  loading = true;

  @property({ type: Object })
  contextHistory: ContextHistory;

  render() {
    return html`
      <style>
        .gitgraph-tooltip {
          position: absolute;
          margin-top: -15px;
          margin-left: 25px;
          padding: 10px;
          border-radius: 5px;
          background: #eee;
          color: #333;
          text-align: center;
          font-size: 14px;
          line-height: 20px;
        }

        .gitgraph-tooltip:after {
          position: absolute;
          top: 10px;
          left: -18px;
          width: 0;
          height: 0;
          border-width: 10px;
          border-style: solid;
          border-color: transparent;
          border-right-color: #eee;
          content: '';
        }

        .gitgraph-detail {
          position: absolute;
          padding: 10px;
          text-align: justify;
          width: 600px;
          display: none;
        }
      </style>
      <script src="js/gitgraph.min.js"></script>

      ${this.loading
        ? html`
            <span>Loading commit history...</span>
          `
        : html`
            <canvas id="gitGraph"></canvas>
          `}
    `;
  }

  protected firstUpdated() {
    this.loadContext();
  }

  loadContext() {
    this.loading = true;
    store
      .dispatch(getContextHistory.create({ context_address: this.context.id }))
      .then(() => {
        this.loading = false;
        setTimeout(() => this.drawHistory());
      });
  }

  update(changedProperties) {
    // Don't forget this or your element won't render!
    super.update(changedProperties);
    if (changedProperties.get('contextId')) {
      this.loadContext();
    }
  }

  stateChanged(state: RootState) {
    this.contextHistory = selectContextHistory(this.context.id)(
      selectVersionControl(state)
    );
  }

  dispatchSelectedEntry(entryId: string) {
    const event = new CustomEvent('entry-selected', {
      detail: {
        entryId: entryId
      }
    });
    this.dispatchEvent(event);
  }

  getBranchesNamesFromCommit(commitId: string) {
    const branches = this.branches.filter(
      branch => branch.branch_head === commitId
    );
    return branches.length > 0
      ? branches.map(branch => branch.name).join(',')
      : '';
  }

  drawHistory() {
    const gitgraph = new GitGraph({
      canvas: this.shadowRoot.getElementById('gitGraph'),
      mode: 'compact',
      orientation: 'vertical-reverse'
    });

    const branchesHeads = this.branches.reduce(
      (branches, branch) => ({ ...branches, [branch.branch_head]: branch }),
      {}
    );

    const originalCommitId = this.contextHistory.originalCommitAddress;
    const masterGraph = gitgraph.branch('');

    const graphHeads: Array<{
      commitId: string;
      branch: GitGraph.Branch;
    }> = [
      {
        commitId: originalCommitId,
        branch: masterGraph
      }
    ];

    console.log('hi', this.contextHistory);
    console.log('hi2', graphHeads);

    for (const commitId of this.contextHistory.ids) {
      console.log('commit', commitId);
      const commit = this.contextHistory.entities[commitId];
      console.log('branch', graphHeads);
      let branchHead = graphHeads.find(branch => branch.commitId === commitId);

      const commitOptions = {
        message: commit.message,
        author: commit.author_address,
        sha1: commit.id
      };

      if (commit.parent_commits_addresses.length <= 1) {
        branchHead.branch.commit(commitOptions);
      } else {
        for (var i = 1; i < commit.parent_commits_addresses.length; i++) {
          let parentBranch = graphHeads.find(
            branch => branch.commitId === commit.parent_commits_addresses[i]
          ).branch;
          branchHead.branch.merge(parentBranch, commitOptions);
        }
      }

      if (commit.children_commits_addresses.length > 0) {
        branchHead.commitId = commit.children_commits_addresses[0];

        if (branchesHeads[commitId]) {
          branchHead.branch = branchHead.branch.branch('');
        }

        for (var i = 1; i < commit.children_commits_addresses.length; i++) {
          const newGraphBranch = branchHead.branch.branch('');
          graphHeads.push({
            commitId: commit.children_commits_addresses[i],
            branch: newGraphBranch
          });
        }
      }

    }
  }
}