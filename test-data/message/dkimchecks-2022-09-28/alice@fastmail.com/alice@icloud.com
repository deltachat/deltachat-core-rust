ARC-Authentication-Results: i=1; mx4.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=qs51p00im-qukt01072701.me.com
    policy.ptr=qs51p00im-qukt01072701.me.com;
    bimi=declined (Domain declined to participate);
    arc=none (no signatures found);
    dkim=pass (2048-bit rsa key sha256) header.d=icloud.com
    header.i=@icloud.com header.b=QwCPOZZR header.a=rsa-sha256
    header.s=1a1hai x-bits=2048;
    dmarc=pass policy.published-domain-policy=quarantine
    policy.published-subdomain-policy=quarantine
    policy.applied-disposition=none policy.evaluated-disposition=none
    (p=quarantine,sp=quarantine,d=none,d.eval=none) policy.policy-from=p
    header.from=icloud.com;
    iprev=pass smtp.remote-ip=17.57.155.16 (qs51p00im-qukt01072701.me.com);
    spf=pass smtp.mailfrom=alice@icloud.com
    smtp.helo=qs51p00im-qukt01072701.me.com
Authentication-Results: mx4.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=qs51p00im-qukt01072701.me.com
      policy.ptr=qs51p00im-qukt01072701.me.com
Authentication-Results: mx4.messagingengine.com;
    bimi=declined (Domain declined to participate)
Authentication-Results: mx4.messagingengine.com;
    arc=none (no signatures found)
Authentication-Results: mx4.messagingengine.com;
    dkim=pass (2048-bit rsa key sha256) header.d=icloud.com
      header.i=@icloud.com header.b=QwCPOZZR header.a=rsa-sha256
      header.s=1a1hai x-bits=2048;
    dmarc=pass policy.published-domain-policy=quarantine
      policy.published-subdomain-policy=quarantine
      policy.applied-disposition=none policy.evaluated-disposition=none
      (p=quarantine,sp=quarantine,d=none,d.eval=none) policy.policy-from=p
      header.from=icloud.com;
    iprev=pass smtp.remote-ip=17.57.155.16 (qs51p00im-qukt01072701.me.com);
    spf=pass smtp.mailfrom=alice@icloud.com
      smtp.helo=qs51p00im-qukt01072701.me.com
From: <alice@icloud.com>
To: <alice@fastmail.com>
