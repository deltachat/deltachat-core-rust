ARC-Authentication-Results: i=1; mx6.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=mail-wm1-f67.google.com
    policy.ptr=mail-wm1-f67.google.com;
    bimi=skipped (DMARC Policy is not at enforcement);
    arc=none (no signatures found);
    dkim=pass (2048-bit rsa key sha256) header.d=gmail.com
    header.i=@gmail.com header.b=hr44hXYS header.a=rsa-sha256
    header.s=20210112 x-bits=2048;
    dmarc=pass policy.published-domain-policy=none
    policy.published-subdomain-policy=quarantine
    policy.applied-disposition=none policy.evaluated-disposition=none
    (p=none,sp=quarantine,d=none,d.eval=none) policy.policy-from=p
    header.from=gmail.com;
    iprev=pass smtp.remote-ip=209.85.128.67 (mail-wm1-f67.google.com);
    spf=pass smtp.mailfrom=alice@gmail.com
    smtp.helo=mail-wm1-f67.google.com
Authentication-Results: mx6.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=mail-wm1-f67.google.com
      policy.ptr=mail-wm1-f67.google.com
Authentication-Results: mx6.messagingengine.com;
    bimi=skipped (DMARC Policy is not at enforcement)
Authentication-Results: mx6.messagingengine.com;
    arc=none (no signatures found)
Authentication-Results: mx6.messagingengine.com;
    dkim=pass (2048-bit rsa key sha256) header.d=gmail.com
      header.i=@gmail.com header.b=hr44hXYS header.a=rsa-sha256
      header.s=20210112 x-bits=2048;
    dmarc=pass policy.published-domain-policy=none
      policy.published-subdomain-policy=quarantine
      policy.applied-disposition=none policy.evaluated-disposition=none
      (p=none,sp=quarantine,d=none,d.eval=none) policy.policy-from=p
      header.from=gmail.com;
    iprev=pass smtp.remote-ip=209.85.128.67 (mail-wm1-f67.google.com);
    spf=pass smtp.mailfrom=alice@gmail.com
      smtp.helo=mail-wm1-f67.google.com
From: <alice@gmail.com>
To: <alice@fastmail.com>
